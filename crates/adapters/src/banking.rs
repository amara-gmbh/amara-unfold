// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::text::{decode_text, sanitize_text, SOURCE_CHARACTER_UNAVAILABLE};
use crate::{
    input,
    model::{date_time_parts, EventInput, OpenEvent},
    record_event, record_skip, AdapterError, ExtractionProgress, ProgressCallback,
};
use calamine::{open_workbook_auto, Data, DataType, Reader};
use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
};

#[derive(Debug, Clone)]
struct BankingTransaction {
    date: String,
    amount: String,
    currency: String,
    counterparty: String,
    description: String,
    account: String,
    raw_path: String,
    raw: String,
}

pub fn extract_workbooks(
    source: &Path,
    package: &crate::writer::PackageWriter,
    cancel: &AtomicBool,
    progress: &mut ExtractionProgress,
    callback: &mut ProgressCallback<'_>,
) -> Result<(), AdapterError> {
    let mut transactions = Vec::new();
    let workbooks = canonical_workbooks(input::files_with_extensions(
        source,
        &["xlsx", "xls", "ods"],
    ));
    for path in workbooks {
        extract_workbook(&path, cancel, progress, callback, &mut transactions)?;
    }
    write_banking_events(transactions, package, progress, callback)
}

pub fn extract_csvs(
    source: &Path,
    package: &crate::writer::PackageWriter,
    cancel: &AtomicBool,
    progress: &mut ExtractionProgress,
    callback: &mut ProgressCallback<'_>,
) -> Result<(), AdapterError> {
    let mut transactions = Vec::new();
    for path in input::files_with_extensions(source, &["csv"]) {
        extract_csv(
            &path,
            cancel,
            progress,
            callback,
            &mut transactions,
            source.is_dir(),
        )?;
    }
    write_banking_events(transactions, package, progress, callback)
}

fn canonical_workbooks(mut paths: Vec<PathBuf>) -> Vec<PathBuf> {
    paths.sort();
    let dated = paths
        .iter()
        .filter(|path| file_name(path).starts_with("b-takeout-"))
        .max_by_key(|path| file_name(path).to_owned())
        .cloned();
    if let Some(path) = dated {
        return vec![path];
    }
    for preferred in ["b-takeout.xlsx", "bank_movements_all.xlsx"] {
        if let Some(path) = paths
            .iter()
            .find(|path| file_name(path).eq_ignore_ascii_case(preferred))
        {
            return vec![path.clone()];
        }
    }
    paths
}

fn extract_workbook(
    path: &Path,
    cancel: &AtomicBool,
    progress: &mut ExtractionProgress,
    callback: &mut ProgressCallback<'_>,
    transactions: &mut Vec<BankingTransaction>,
) -> Result<(), AdapterError> {
    input::validate_file_size(path, 2 * 1024 * 1024 * 1024, "bank workbook")?;
    if matches!(
        path.extension().and_then(|extension| extension.to_str()),
        Some(extension) if extension.eq_ignore_ascii_case("xlsx") || extension.eq_ignore_ascii_case("ods")
    ) {
        input::validate_zip_container(
            path,
            input::MAX_ARCHIVE_ENTRIES,
            input::MAX_BLOB_BYTES,
            input::MAX_ARCHIVE_TOTAL_BYTES,
        )?;
    }
    let mut workbook = open_workbook_auto(path)
        .map_err(|error| AdapterError::Parse(format!("{}: {error}", path.display())))?;
    let names = workbook.sheet_names().to_vec();
    let candidates = names
        .iter()
        .find(|name| name.eq_ignore_ascii_case("Readable_Ledger"))
        .cloned()
        .into_iter()
        .chain(
            names
                .into_iter()
                .filter(|name| !name.eq_ignore_ascii_case("Readable_Ledger")),
        )
        .collect::<Vec<_>>();
    for sheet_name in candidates {
        let range = workbook
            .worksheet_range(&sheet_name)
            .map_err(|error| AdapterError::Parse(error.to_string()))?;
        let rows = range.rows().collect::<Vec<_>>();
        let Some((header_index, indexes)) =
            rows.iter().take(50).enumerate().find_map(|(index, row)| {
                let headers = row.iter().map(workbook_cell_text).collect::<Vec<_>>();
                let indexes = header_indexes(&headers);
                usable_headers(&indexes).then_some((index, indexes))
            })
        else {
            continue;
        };
        let data_rows = rows.len().saturating_sub(header_index + 1);
        progress.total_hint = Some(progress.total_hint.unwrap_or(0) + data_rows as u64);
        progress.message = format!("Reading {data_rows} transaction rows from {sheet_name}");
        callback(progress.clone());
        for (row_index, row) in rows.into_iter().enumerate().skip(header_index + 1) {
            if cancel.load(Ordering::Relaxed) {
                break;
            }
            let values = row.iter().map(workbook_cell_text).collect::<Vec<_>>();
            collect_row(
                &values,
                &indexes,
                &format!("{}#{}#row-{}", path.display(), sheet_name, row_index + 1),
                progress,
                transactions,
            );
            if row_index % 2_000 == 0 {
                progress.message = format!(
                    "Reading transactions · {} usable rows collected",
                    transactions.len()
                );
                callback(progress.clone());
            }
        }
        // Readable_Ledger is the canonical sheet; summary and filtered views duplicate it.
        break;
    }
    Ok(())
}

fn workbook_cell_text(cell: &Data) -> String {
    match cell {
        Data::DateTime(value) if !value.is_duration() => value
            .as_datetime()
            .map(|date| date.format("%Y-%m-%d %H:%M:%S").to_string())
            .unwrap_or_default(),
        Data::DateTimeIso(value) => value.clone(),
        _ => cell.as_string().unwrap_or_default(),
    }
}

fn extract_csv(
    path: &Path,
    cancel: &AtomicBool,
    progress: &mut ExtractionProgress,
    callback: &mut ProgressCallback<'_>,
    transactions: &mut Vec<BankingTransaction>,
    allow_unrelated: bool,
) -> Result<(), AdapterError> {
    input::validate_file_size(path, input::MAX_BLOB_BYTES, "bank CSV")?;
    let bytes = std::fs::read(path).map_err(|error| AdapterError::Input(error.to_string()))?;
    let text = decode_text(&bytes);
    let Some(delimiter) = header_delimiter(&text) else {
        if allow_unrelated {
            return Ok(());
        }
        return Err(AdapterError::Parse(format!(
            "{}: no transaction header row found",
            path.display()
        )));
    };
    let mut reader = csv::ReaderBuilder::new()
        .delimiter(delimiter)
        .has_headers(false)
        .flexible(true)
        .from_reader(text.as_bytes());
    let rows = reader
        .records()
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| AdapterError::Parse(format!("{}: {error}", path.display())))?;
    let Some((header_index, indexes)) = rows.iter().enumerate().find_map(|(index, row)| {
        let headers = row.iter().map(str::to_owned).collect::<Vec<_>>();
        let indexes = header_indexes(&headers);
        usable_headers(&indexes).then_some((index, indexes))
    }) else {
        return Err(AdapterError::Parse(format!(
            "{}: no transaction header row found",
            path.display()
        )));
    };
    progress.total_hint =
        Some(progress.total_hint.unwrap_or(0) + rows.len().saturating_sub(header_index + 1) as u64);
    callback(progress.clone());
    for (row_index, row) in rows.into_iter().enumerate().skip(header_index + 1) {
        if cancel.load(Ordering::Relaxed) {
            break;
        }
        let values = row.iter().map(str::to_owned).collect::<Vec<_>>();
        collect_row(
            &values,
            &indexes,
            &format!("{}#row-{}", path.display(), row_index + 1),
            progress,
            transactions,
        );
        if row_index % 2_000 == 0 {
            progress.message = format!(
                "Reading transactions · {} usable rows collected",
                transactions.len()
            );
            callback(progress.clone());
        }
    }
    Ok(())
}

fn collect_row(
    row: &[String],
    indexes: &HashMap<String, usize>,
    raw_path: &str,
    progress: &mut ExtractionProgress,
    transactions: &mut Vec<BankingTransaction>,
) {
    if row.iter().all(|value| value.trim().is_empty()) {
        return;
    }
    let date_text = value(row, indexes, date_aliases());
    if date_text.is_empty() {
        progress.records_seen += 1;
        record_skip(progress, "missing_date");
        return;
    }
    let Some((date, _time)) = banking_date_time_parts(&date_text) else {
        eprintln!("banking: skipped invalid date `{date_text}` at {raw_path}");
        progress.records_seen += 1;
        record_skip(progress, "invalid_date");
        progress.errors += 1;
        return;
    };
    let amount = transaction_amount(row, indexes);
    if amount.is_empty() {
        progress.records_seen += 1;
        record_skip(progress, "missing_amount");
        return;
    }
    let raw_counterparty = value(
        row,
        indexes,
        &[
            "counterparty",
            "merchant",
            "payee",
            "name",
            "empfänger",
            "empfaenger",
            "auftraggeber",
            "auftraggeber/empfänger",
            "auftraggeber/empfaenger",
            "beguenstigter/zahlungspflichtiger",
            "zahlungsempfänger",
            "zahlungsempfaenger",
            "recipient",
            "beneficiary",
        ],
    );
    let description = value(
        row,
        indexes,
        &[
            "description",
            "purpose",
            "memo",
            "verwendungszweck",
            "categoryhint",
            "buchungstext",
            "transaction details",
            "details",
            "reference",
            "note",
        ],
    );
    let purpose = value(row, indexes, &["purpose", "verwendungszweck"]);
    let booking_text = value(row, indexes, &["bookingtext", "buchungstext"]);
    let counterparty =
        normalize_counterparty(&raw_counterparty, &description, &purpose, &booking_text);
    let currency = value(
        row,
        indexes,
        &["currency", "währung", "waehrung", "ccy", "currency code"],
    );
    let account = value(
        row,
        indexes,
        &[
            "account",
            "sourceaccount",
            "source account",
            "konto",
            "auftragskonto",
            "account name",
            "account number",
            "iban",
        ],
    );
    transactions.push(BankingTransaction {
        date,
        amount,
        currency,
        counterparty,
        description: combined_description(&description, &purpose, &booking_text),
        account,
        raw_path: raw_path.to_owned(),
        raw: row.join("\t"),
    });
    progress.source_records_included += 1;
}

fn usable_headers(indexes: &HashMap<String, usize>) -> bool {
    find_index(indexes, date_aliases()).is_some()
        && (find_index(indexes, amount_aliases()).is_some()
            || find_index(indexes, debit_aliases()).is_some()
            || find_index(indexes, credit_aliases()).is_some())
}

fn date_aliases() -> &'static [&'static str] {
    &[
        "date",
        "bookingdate",
        "booking date",
        "transaction date",
        "value date",
        "valuedate",
        "buchungstag",
        "buchungsdatum",
        "wertstellung",
        "valuta",
        "buchung",
        "datum",
    ]
}

fn amount_aliases() -> &'static [&'static str] {
    &[
        "amount",
        "transaction amount",
        "booking amount",
        "betrag",
        "value",
        "umsatz",
        "sum",
    ]
}

fn debit_aliases() -> &'static [&'static str] {
    &["debit", "debit amount", "soll", "withdrawal", "belastung"]
}

fn credit_aliases() -> &'static [&'static str] {
    &["credit", "credit amount", "haben", "deposit", "gutschrift"]
}

fn transaction_amount(row: &[String], indexes: &HashMap<String, usize>) -> String {
    let amount = value(row, indexes, amount_aliases());
    if !amount.is_empty() {
        return amount;
    }
    let debit = value(row, indexes, debit_aliases());
    if !debit.is_empty() {
        return if debit.trim_start().starts_with('-') {
            debit
        } else {
            format!("-{debit}")
        };
    }
    value(row, indexes, credit_aliases())
}

fn combined_description(description: &str, purpose: &str, booking_text: &str) -> String {
    let mut parts: Vec<String> = Vec::new();
    for value in [description, purpose, booking_text] {
        let value = collapse_whitespace(value);
        if value.is_empty() {
            continue;
        }
        let lower = value.to_lowercase();
        if parts
            .iter()
            .any(|existing| existing.to_lowercase().contains(&lower))
        {
            continue;
        }
        parts.retain(|existing| !lower.contains(&existing.to_lowercase()));
        parts.push(value);
    }
    parts.join(" · ")
}

fn write_banking_events(
    transactions: Vec<BankingTransaction>,
    package: &crate::writer::PackageWriter,
    progress: &mut ExtractionProgress,
    callback: &mut ProgressCallback<'_>,
) -> Result<(), AdapterError> {
    let transaction_count = transactions.len();
    let mut by_day = BTreeMap::<String, Vec<BankingTransaction>>::new();
    let mut by_month = BTreeMap::<String, Vec<BankingTransaction>>::new();
    for transaction in transactions {
        let month = transaction.date.get(..7).unwrap_or("0000-00").to_owned();
        by_month.entry(month).or_default().push(transaction.clone());
        by_day
            .entry(transaction.date.clone())
            .or_default()
            .push(transaction);
    }
    progress.total_hint = Some(progress.records_seen + (by_day.len() + by_month.len()) as u64);
    progress.message = format!(
        "Creating {} daily movement files and {} monthly overviews from {transaction_count} transactions",
        by_day.len(),
        by_month.len(),
    );
    callback(progress.clone());

    for (date, mut rows) in by_day {
        sort_transactions(&mut rows);
        let title = format!("Banking movements — {date}");
        let raw_path = grouped_raw_path(&rows, "daily-movements", &date);
        let raw = grouped_raw_evidence(&rows);
        let body = daily_movements_body(&rows);
        let event = OpenEvent::new(EventInput {
            date: &date,
            time: None,
            end_date: None,
            source: "banking",
            module: "banking",
            event_type: "daily_movements",
            title: &title,
            raw_path: &raw_path,
            raw_bytes: raw.as_bytes(),
            body,
            actors: Vec::new(),
            places: Vec::new(),
            tags: vec!["banking".into(), "daily-movements".into()],
            confidence: "high",
        });
        record_event(package, &event, progress, callback)?;
    }

    for (month, mut rows) in by_month {
        sort_transactions(&mut rows);
        let title = format!("Banking overview — {month}");
        let raw_path = grouped_raw_path(&rows, "monthly-overview", &month);
        let raw = grouped_raw_evidence(&rows);
        let body = monthly_overview_body(&rows);
        let event = OpenEvent::new(EventInput {
            date: &month,
            time: None,
            end_date: None,
            source: "banking",
            module: "banking",
            event_type: "monthly_overview",
            title: &title,
            raw_path: &raw_path,
            raw_bytes: raw.as_bytes(),
            body,
            actors: Vec::new(),
            places: Vec::new(),
            tags: vec!["banking".into(), "monthly-overview".into()],
            confidence: "high",
        });
        record_event(package, &event, progress, callback)?;
    }
    Ok(())
}

fn sort_transactions(rows: &mut [BankingTransaction]) {
    rows.sort_by(|left, right| {
        left.date
            .cmp(&right.date)
            .then_with(|| left.account.cmp(&right.account))
            .then_with(|| left.counterparty.cmp(&right.counterparty))
            .then_with(|| left.amount.cmp(&right.amount))
            .then_with(|| left.description.cmp(&right.description))
            .then_with(|| left.raw_path.cmp(&right.raw_path))
    });
}

fn grouped_raw_path(rows: &[BankingTransaction], kind: &str, period: &str) -> String {
    let sources = rows
        .iter()
        .filter_map(|row| row.raw_path.split('#').next())
        .collect::<BTreeSet<_>>();
    if sources.len() == 1 {
        format!(
            "{}#{kind}-{period}",
            sources.into_iter().next().unwrap_or_default()
        )
    } else {
        format!("{} source files#{kind}-{period}", sources.len())
    }
}

fn grouped_raw_evidence(rows: &[BankingTransaction]) -> String {
    rows.iter()
        .map(|row| format!("{}\t{}", row.raw_path, row.raw))
        .collect::<Vec<_>>()
        .join("\n")
}

fn daily_movements_body(rows: &[BankingTransaction]) -> String {
    let source_files = rows
        .iter()
        .filter_map(|row| row.raw_path.split('#').next())
        .collect::<BTreeSet<_>>();
    let mut lines = vec![format!(
        "{} banking {} recorded for this day.",
        rows.len(),
        if rows.len() == 1 {
            "movement was"
        } else {
            "movements were"
        },
    )];
    for (index, row) in rows.iter().enumerate() {
        let target = display_value(&row.counterparty);
        let currency = collapse_whitespace(&row.currency);
        let amount = collapse_whitespace(&row.amount);
        let heading = if currency.is_empty() {
            format!("## Movement {} · {} — {target}", index + 1, amount)
        } else {
            format!(
                "## Movement {} · {} {} — {target}",
                index + 1,
                amount,
                currency
            )
        };
        lines.push(String::new());
        lines.push(heading);
        if !row.account.is_empty() {
            lines.push(format!(
                "- Account / source: {}",
                safe_markdown_text(&row.account)
            ));
        }
        if !row.description.is_empty() {
            lines.push(format!(
                "- Description: {}",
                safe_markdown_text(&row.description)
            ));
        }
    }
    lines.push(String::new());
    lines.push("## Source evidence".into());
    for source in source_files {
        lines.push(format!("- `{}`", source.replace('`', "'")));
    }
    lines.join("\n")
}

fn monthly_overview_body(rows: &[BankingTransaction]) -> String {
    let mut days = BTreeMap::<&str, usize>::new();
    for row in rows {
        *days.entry(&row.date).or_default() += 1;
    }
    let mut lines = vec![format!(
        "{} banking movements across {} active {}. Daily movement files contain the details.",
        rows.len(),
        days.len(),
        if days.len() == 1 { "day" } else { "days" },
    )];
    for (date, count) in days {
        lines.push(format!(
            "- {date} — {count} {}",
            if count == 1 { "movement" } else { "movements" }
        ));
    }
    lines.join("\n")
}

fn display_value(value: &str) -> String {
    if value.is_empty() {
        "Unspecified target".into()
    } else {
        safe_markdown_text(value)
    }
}

fn safe_markdown_text(value: &str) -> String {
    collapse_whitespace(value).replace(['\r', '\n'], " ")
}

pub(crate) fn header_delimiter(text: &str) -> Option<u8> {
    (*b";,\t|").into_iter().find(|delimiter| {
        csv::ReaderBuilder::new()
            .delimiter(*delimiter)
            .has_headers(false)
            .flexible(true)
            .from_reader(text.trim_start_matches('\u{feff}').as_bytes())
            .records()
            .take(50)
            .filter_map(Result::ok)
            .any(|row| {
                usable_headers(&header_indexes(
                    &row.iter().map(str::to_owned).collect::<Vec<_>>(),
                ))
            })
    })
}

fn banking_date_time_parts(value: &str) -> Option<(String, Option<String>)> {
    let trimmed = value.trim();
    let (date_value, explicit_time) = trimmed
        .split_once(' ')
        .map(|(date, time)| {
            let time = time
                .trim()
                .get(..5)
                .filter(|value| value.chars().nth(2) == Some(':'))
                .map(str::to_owned);
            (date, time)
        })
        .unwrap_or((trimmed, None));
    let expanded = expand_two_digit_year(date_value).unwrap_or_else(|| date_value.to_owned());
    date_time_parts(&expanded).map(|(date, parsed_time)| (date, explicit_time.or(parsed_time)))
}

fn expand_two_digit_year(value: &str) -> Option<String> {
    let separator = if value.matches('.').count() == 2 {
        '.'
    } else if value.matches('/').count() == 2 {
        '/'
    } else {
        return None;
    };
    let parts = value.split(separator).collect::<Vec<_>>();
    if parts.len() != 3 || parts[2].len() != 2 {
        return None;
    }
    let year = parts[2].parse::<u16>().ok()?;
    let year = if year >= 90 { 1900 + year } else { 2000 + year };
    Some(format!(
        "{}{}{}{}{}",
        parts[0], separator, parts[1], separator, year
    ))
}

fn header_indexes(headers: &[String]) -> HashMap<String, usize> {
    headers
        .iter()
        .enumerate()
        .map(|(index, name)| {
            (
                sanitize_text(name)
                    .trim()
                    .trim_start_matches('\u{feff}')
                    .to_lowercase()
                    .replace(['_', '-'], " ")
                    .split_whitespace()
                    .collect::<Vec<_>>()
                    .join(" "),
                index,
            )
        })
        .collect()
}

fn find_index(indexes: &HashMap<String, usize>, aliases: &[&str]) -> Option<usize> {
    aliases
        .iter()
        .find_map(|alias| indexes.get(*alias).copied())
}

fn value(row: &[String], indexes: &HashMap<String, usize>, aliases: &[&str]) -> String {
    find_index(indexes, aliases)
        .and_then(|index| row.get(index))
        .map(|value| sanitize_text(value).trim().to_owned())
        .unwrap_or_default()
}

fn normalize_counterparty(raw: &str, description: &str, purpose: &str, booking: &str) -> String {
    let combined = [raw, description, purpose, booking].join(" ");
    if is_atm_withdrawal(&combined) {
        return String::new();
    }
    if let Some(merchant) = known_merchant(&combined) {
        return merchant.into();
    }

    let cleaned = if is_machine_noise(raw) {
        String::new()
    } else {
        clean_party_candidate(raw)
    };
    if is_party_candidate(&cleaned) {
        return canonical_display(&cleaned);
    }

    for context in [description, purpose, booking] {
        let cleaned = clean_party_candidate(context);
        if is_context_party_candidate(&cleaned) {
            return canonical_display(&cleaned);
        }
    }
    String::new()
}

fn known_merchant(value: &str) -> Option<&'static str> {
    let upper = value.to_uppercase();
    let merchants: &[(&[&str], &str)] = &[
        (&["AIRBNB"], "Airbnb"),
        (&["AMZN", "AMAZON"], "Amazon"),
        (&["APPLE.COM", "APPLE STORE", "APPLE PAYMENTS"], "Apple"),
        (&["NETFLIX"], "Netflix"),
        (&["DISNEY"], "Disney"),
        (&["PAYPAL"], "PayPal"),
        (&["UBER"], "Uber"),
        (&["BOOKING.COM", "BOOKING DOT COM"], "Booking.com"),
        (&["SPOTIFY"], "Spotify"),
        (&["YOUTUBE"], "YouTube"),
        (&["MICROSOFT"], "Microsoft"),
        (&["OPENAI"], "OpenAI"),
        (&["ANTHROPIC"], "Anthropic"),
        (&["STEAMGAMES", "STEAM GAMES"], "Steam"),
        (&["PLAYSTATION"], "PlayStation"),
        (&["REWE"], "REWE"),
        (&["ALDI"], "ALDI"),
        (&["LIDL"], "LIDL"),
        (&["EDEKA"], "EDEKA"),
        (&["DM-DROGERIE", "DM DROGERIE"], "DM"),
        (&["ROSSMANN"], "Rossmann"),
        (&["ALLIANZ"], "Allianz"),
        (&["JET-TANK", "JET TANK"], "JET"),
        (&["ARAL"], "ARAL"),
        (&["SHELL"], "Shell"),
    ];
    merchants.iter().find_map(|(needles, name)| {
        needles
            .iter()
            .any(|needle| upper.contains(needle))
            .then_some(*name)
    })
}

fn clean_party_candidate(value: &str) -> String {
    let mut cleaned = collapse_whitespace(&sanitize_text(value));
    if cleaned.contains(SOURCE_CHARACTER_UNAVAILABLE) {
        return String::new();
    }
    cleaned = dedupe_repeated_text(&cleaned);
    if let Some((before, _)) = cleaned.split_once("//") {
        cleaned = before.trim().to_owned();
    }
    for prefix in ["VISA ", "MASTERCARD ", "MAESTRO ", "DEBITKARTE "] {
        if cleaned.to_uppercase().starts_with(prefix) {
            cleaned = cleaned[prefix.len()..].trim().to_owned();
            break;
        }
    }
    let words = cleaned.split_whitespace().collect::<Vec<_>>();
    if words.first().is_some_and(|word| {
        word.len() >= 3 && word.len() <= 6 && word.chars().all(|ch| ch.is_ascii_digit())
    }) {
        cleaned = words[1..].join(" ");
    }
    let mut words = cleaned.split_whitespace().collect::<Vec<_>>();
    if let Some(index) = words
        .iter()
        .position(|word| word.chars().filter(|ch| ch.is_ascii_digit()).count() >= 6)
    {
        words.truncate(index);
        cleaned = words.join(" ");
    }
    let upper = cleaned.to_uppercase();
    if let Some(index) = upper.find(" SAGT DANKE") {
        cleaned.truncate(index);
    }
    collapse_whitespace(cleaned.trim_matches([' ', ',', ';', '-', '.']))
}

fn dedupe_repeated_text(value: &str) -> String {
    let characters = value.chars().collect::<Vec<_>>();
    if characters.len() >= 8 && characters.len() % 2 == 0 {
        let middle = characters.len() / 2;
        let left = characters[..middle].iter().collect::<String>();
        let right = characters[middle..].iter().collect::<String>();
        if left.eq_ignore_ascii_case(&right) {
            return left.trim().to_owned();
        }
    }
    value.to_owned()
}

fn is_party_candidate(value: &str) -> bool {
    if value.is_empty() || is_machine_noise(value) {
        return false;
    }
    let letters = value
        .chars()
        .filter(|character| character.is_alphabetic())
        .count();
    letters >= 2 && letters * 2 >= value.chars().count()
}

fn is_context_party_candidate(value: &str) -> bool {
    if !is_party_candidate(value) || value.len() > 80 {
        return false;
    }
    let upper = value.to_uppercase();
    ![
        "ABSCHLUSS",
        "DAUERAUFTRAG",
        "ENTGELT",
        "GUTSCHRIFT",
        "KARTENZAHLUNG",
        "LASTSCHRIFT",
        "ONLINE-BANKING",
        "SEPA",
        "ÜBERWEISUNG",
        "UEBERWEISUNG",
        "ZINSEN",
    ]
    .iter()
    .any(|needle| upper.contains(needle))
}

fn is_machine_noise(value: &str) -> bool {
    let upper = collapse_whitespace(&value.to_uppercase());
    matches!(upper.as_str(), "GA" | "ATM" | "GELDAUTOMAT")
        || upper.starts_with("GA NR")
        || upper.contains(" BLZ")
        || upper.starts_with("BLZ")
        || upper.contains(" IBAN")
        || upper.contains(" BIC")
        || value.chars().all(|character| {
            character.is_ascii_digit()
                || character.is_ascii_punctuation()
                || character.is_whitespace()
        })
}

fn is_atm_withdrawal(value: &str) -> bool {
    let upper = format!(" {} ", collapse_whitespace(&value.to_uppercase()));
    upper.contains(" GA NR")
        || upper.contains(" GELDAUTOMAT")
        || upper.contains(" BARGELDAUSZAHLUNG")
        || upper.contains(" BARAUSZAHLUNG")
        || upper.contains(" CASH WITHDRAWAL")
        || upper.contains(" ATM ")
        || upper.trim() == "GA"
}

fn canonical_display(value: &str) -> String {
    let mut value = dedupe_repeated_text(&collapse_whitespace(value));
    if value.matches(',').count() == 1 {
        if let Some((last, first)) = value.split_once(',') {
            if last.split_whitespace().count() == 1
                && first.split_whitespace().count() == 1
                && last
                    .trim()
                    .chars()
                    .all(|character| character.is_alphabetic())
                && first
                    .trim()
                    .chars()
                    .all(|character| character.is_alphabetic())
            {
                value = format!("{} {}", first.trim(), last.trim());
            }
        }
    }
    let letters = value
        .chars()
        .filter(|character| character.is_alphabetic())
        .collect::<Vec<_>>();
    if !letters.is_empty() && letters.iter().all(|character| character.is_uppercase()) {
        value = value
            .split_whitespace()
            .map(title_case_party_word)
            .collect::<Vec<_>>()
            .join(" ");
    }
    value.chars().take(100).collect()
}

fn title_case_party_word(value: &str) -> String {
    match value.trim_matches(['.', ',']).to_uppercase().as_str() {
        "AG" | "GMBH" | "KG" | "OHG" | "UG" => return value.to_uppercase(),
        _ => {}
    }
    let mut characters = value.chars();
    let Some(first) = characters.next() else {
        return String::new();
    };
    first
        .to_uppercase()
        .chain(characters.flat_map(char::to_lowercase))
        .collect()
}

fn collapse_whitespace(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    #[test]
    fn workbook_dates_keep_type_and_epoch_without_reinterpreting_numbers() {
        use calamine::{Data, ExcelDateTime, ExcelDateTimeType};
        for (serial, epoch1904) in [(45387.0, false), (43925.0, true)] {
            let cell = Data::DateTime(ExcelDateTime::new(
                serial,
                ExcelDateTimeType::DateTime,
                epoch1904,
            ));
            assert_eq!(super::workbook_cell_text(&cell), "2024-04-05 00:00:00");
        }
        assert_eq!(
            super::workbook_cell_text(&Data::DateTimeIso("2024-04-06T00:00:00".into())),
            "2024-04-06T00:00:00"
        );
        assert_eq!(super::workbook_cell_text(&Data::Float(-12.5)), "-12.5");
        assert_eq!(super::workbook_cell_text(&Data::Float(45387.0)), "45387");
        assert_eq!(super::workbook_cell_text(&Data::Empty), "");
    }

    use super::*;

    #[test]
    fn descriptions_drop_values_already_contained_in_a_richer_field() {
        assert_eq!(
            combined_description(
                "EXAMPLE PROVIDER, INC, EXAMPLE CITY - Credit card PDF statement",
                "EXAMPLE PROVIDER, INC, EXAMPLE CITY",
                "Credit card PDF statement",
            ),
            "EXAMPLE PROVIDER, INC, EXAMPLE CITY - Credit card PDF statement"
        );
        assert_eq!(
            combined_description("Invoice 12", "Different purpose", ""),
            "Invoice 12 · Different purpose"
        );
    }

    #[test]
    fn counterparty_normalization_removes_machine_noise_and_taglines() {
        assert_eq!(
            normalize_counterparty("GA NR123 BLZ456 0", "", "", ""),
            ""
        );
        assert_eq!(normalize_counterparty("99999999", "", "", ""), "");
        assert_eq!(normalize_counterparty("Ga", "", "", ""), "");
        assert_eq!(
            normalize_counterparty("Ga", "GA NR123 BLZ456 0", "", ""),
            ""
        );
        assert_eq!(
            normalize_counterparty("Corner Cafe", "ATM CASH WITHDRAWAL", "", ""),
            ""
        );
        assert_eq!(
            normalize_counterparty("ALDI SUED SAGT DANKE", "", "", ""),
            "ALDI"
        );
        assert_eq!(
            normalize_counterparty("123 EDEKA TEST LOCATION", "", "", ""),
            "EDEKA"
        );
        assert_eq!(
            normalize_counterparty("JET-TANK. 1234, TEST LOCATION", "", "", ""),
            "JET"
        );
    }

    #[test]
    fn counterparty_normalization_deduplicates_and_uses_clean_context() {
        assert_eq!(
            normalize_counterparty("EXAMPLE GMBHEXAMPLE GMBH", "", "", ""),
            "Example GMBH"
        );
        assert_eq!(
            normalize_counterparty("", "Corner Cafe // terminal 12345678", "", ""),
            "Corner Cafe"
        );
        assert_eq!(normalize_counterparty("DOE, JANE", "", "", ""), "Jane Doe");
        assert_eq!(
            normalize_counterparty("EXAMPLE PROVIDER, INC, EXAMPLE CITY", "", "", ""),
            "Example Provider, Inc, Example City"
        );
        assert_eq!(
            normalize_counterparty("EXAMPLE.AI SUBSCRIPTION, EXAMPLE CITY", "", "", ""),
            "Example.ai Subscription, Example City"
        );
    }

    #[test]
    fn banking_dates_expand_two_digit_years_before_event_identity_is_created() {
        assert_eq!(
            banking_date_time_parts("31.12.24"),
            Some(("2024-12-31".into(), None))
        );
        assert_eq!(
            banking_date_time_parts("01.01.99 08:45"),
            Some(("1999-01-01".into(), Some("08:45".into())))
        );
        assert_eq!(
            banking_date_time_parts("2026-07-20"),
            Some(("2026-07-20".into(), None))
        );
    }
}
