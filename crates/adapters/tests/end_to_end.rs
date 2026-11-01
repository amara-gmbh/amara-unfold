// SPDX-License-Identifier: AGPL-3.0-or-later

use amara_unfold_adapters::{extract, ExtractionRequest, EXECUTABLE_ADAPTERS};
use std::{
    fs,
    io::{Cursor, Write},
    path::{Path, PathBuf},
    sync::atomic::AtomicBool,
};
use tempfile::TempDir;
use zip::{write::SimpleFileOptions, ZipWriter};

#[test]
fn registered_release_routes_emit_portable_markdown_end_to_end() {
    let root = TempDir::new().unwrap();
    let output = root.path().join("output");
    fs::create_dir(&output).unwrap();

    let fixtures = [
        ("photo-media-folder", photo_fixture(root.path())),
        ("banking-xlsx", banking_fixture(root.path())),
        ("banking-csv", banking_csv_fixture(root.path())),
        ("airbnb-json-takeout", airbnb_fixture(root.path())),
        ("diary-docx", diary_fixture(root.path())),
        ("gmail-takeout", gmail_fixture(root.path())),
        ("apple-mail-local", apple_mail_fixture(root.path())),
        ("apple-calendar-local", apple_calendar_fixture(root.path())),
        ("apple-contacts-local", apple_contacts_fixture(root.path())),
        ("youtube-takeout", youtube_fixture(root.path())),
        ("google-photos-takeout", google_photos_fixture(root.path())),
        ("google-maps", kmz_fixture(root.path())),
        ("apple-music-tsv", apple_music_fixture(root.path())),
        (
            "apple-music-local-library",
            apple_music_local_fixture(root.path()),
        ),
    ];

    assert_eq!(EXECUTABLE_ADAPTERS.len(), 14);
    let uncovered = EXECUTABLE_ADAPTERS
        .iter()
        .filter(|adapter| !fixtures.iter().any(|(covered, _)| *adapter == covered))
        .copied()
        .collect::<Vec<_>>();
    assert!(uncovered.is_empty());
    for (adapter_id, source) in fixtures {
        let result = extract(
            ExtractionRequest {
                adapter_id: adapter_id.into(),
                source,
                destination: output.clone(),
            },
            &AtomicBool::new(false),
            &mut |_| {},
        )
        .unwrap_or_else(|error| panic!("{adapter_id} failed: {error}"));
        if adapter_id == "apple-music-local-library" {
            assert_eq!(result.events_written, 0);
            assert!(Path::new(&result.folder)
                .join("APPLE-MUSIC-OVERVIEW.md")
                .is_file());
            assert!(Path::new(&result.folder).join("library/a.md").is_file());
        } else {
            assert!(result.events_written > 0, "{adapter_id} wrote no events");
            verify_open_package(Path::new(&result.folder), adapter_id);
        }
    }
}

#[test]
fn google_maps_formats_converge_on_one_provider_without_manual_unpacking() {
    let root = TempDir::new().unwrap();
    let fixtures = [
        (
            "standalone-kmz",
            standalone_kmz_fixture(root.path()),
            ["module: google-maps", "type: saved-place", "- reference"].as_slice(),
        ),
        (
            "zipped-csv",
            maps_csv_zip_fixture(root.path()),
            [
                "module: google-maps",
                "Google Maps URL: https://maps.example/library",
                "- reference",
            ]
            .as_slice(),
        ),
        (
            "takeout-geojson",
            maps_geojson_fixture(root.path()),
            [
                "date: \"2023-04-12\"",
                "module: google-maps",
                "Address: 42 Example Street, Example City",
                "Google Maps URL: http://maps.google.com/?cid=123456",
                "Published: 2023-04-12T12:45:44Z",
            ]
            .as_slice(),
        ),
    ];

    for (name, source, expected) in fixtures {
        let output = root.path().join(format!("output-{name}"));
        fs::create_dir(&output).unwrap();
        let result = extract(
            ExtractionRequest {
                adapter_id: "google-maps".into(),
                source,
                destination: output,
            },
            &AtomicBool::new(false),
            &mut |_| {},
        )
        .unwrap_or_else(|error| panic!("{name} failed: {error}"));
        assert!(result.events_written > 0, "{name} wrote no events");
        assert_eq!(result.quality.status, "pass");
        let markdown = collect_markdown(Path::new(&result.folder).join("events"));
        for needle in expected {
            assert!(
                markdown.contains(needle),
                "{name} missing {needle:?}: {markdown}"
            );
        }
    }
}

#[test]
fn apple_music_emits_compact_daily_activity_and_rejects_sentinel_dates() {
    let root = TempDir::new().unwrap();
    let output = root.path().join("output");
    fs::create_dir(&output).unwrap();
    let source = directory(root.path(), "apple-music-compact-source");
    fs::create_dir(source.join("raw")).unwrap();
    fs::write(
        source.join("raw/library_tracks.tsv"),
        "Name\tArtist\tAlbum\tGenre\tDateAdded\tLastPlayed\tPlayCount\tPersistentID\nFirst Song\tOpen Artist\tOpen Album\tRock\t2024-01-02T10:00\t1904-01-01T02:00\t0\tONE\nSecond Song\tOpen Artist\tOpen Album\tRock\t2024-01-02T11:00\t2024-01-03T12:00\t4\tTWO\n",
    )
    .unwrap();

    let request = ExtractionRequest {
        adapter_id: "apple-music-tsv".into(),
        source: source.clone(),
        destination: output.clone(),
    };
    let first = extract(request.clone(), &AtomicBool::new(false), &mut |_| {}).unwrap();

    assert_eq!(first.source_records_included, 2);
    assert_eq!(first.events_written, 1); // one genuine Date Added day only
    assert_eq!(first.events_skipped, 0);
    assert_eq!(first.quality.status, "pass");
    let package = Path::new(&first.folder);
    let events = collect_markdown(package.join("events"));
    let catalog = fs::read_to_string(package.join("library/o.md")).unwrap();
    let coverage = fs::read_to_string(package.join("APPLE-MUSIC-COVERAGE.md")).unwrap();
    assert!(events.contains("Added to library: 2"));
    assert!(events.contains("**First Song** — **Open Artist** · Open Album"));
    assert!(!events.contains("Latest known play"));
    assert!(!events.contains("2024-01-03"));
    assert!(!events.contains("1904-01-01"));
    assert!(!catalog.contains("1904-01-01"));
    assert!(catalog.contains("invalid/sentinel — excluded"));
    assert!(coverage.contains("| Invalid/sentinel Last Played values excluded | 1 |"));

    let rerun = extract(request, &AtomicBool::new(false), &mut |_| {}).unwrap();
    assert_eq!(rerun.events_written, 0);
    assert_eq!(rerun.events_skipped, 1);
    assert_eq!(rerun.skip_reasons.get("unchanged_rerun"), Some(&1));
}

#[test]
fn google_maps_cross_format_duplicates_are_written_once_without_title_guessing() {
    let root = TempDir::new().unwrap();
    let source = directory(root.path(), "maps-overlap");
    fs::write(
        source.join("same-place.kmz"),
        zip_bytes(&[(
            "doc.kml",
            r#"<?xml version="1.0"?><kml xmlns="http://www.opengis.net/kml/2.2"><Document><Placemark><name>Open Library</name><Point><coordinates>11.5,48.1,0</coordinates></Point></Placemark></Document></kml>"#,
        )]),
    )
    .unwrap();
    fs::write(
        source.join("ALL_Locations.csv"),
        "Title,Google Maps URL\nOpen Library,https://www.google.com/maps/place/Open+Library/data=!4m2!3m1!1s0xabc:0x1e240\n",
    )
    .unwrap();
    let maps_folder = source.join("Takeout/Maps (Your Places)");
    fs::create_dir_all(&maps_folder).unwrap();
    fs::write(
        maps_folder.join("Saved Places.json"),
        r#"{"type":"FeatureCollection","features":[{"geometry":{"coordinates":[11.5,48.1],"type":"Point"},"properties":{"Google Maps URL":"https://maps.google.com/?cid=123456","Title":"Open Library"},"type":"Feature"}]}"#,
    )
    .unwrap();
    let output = root.path().join("maps-overlap-output");
    fs::create_dir(&output).unwrap();

    let result = extract(
        ExtractionRequest {
            adapter_id: "google-maps".into(),
            source,
            destination: output,
        },
        &AtomicBool::new(false),
        &mut |_| {},
    )
    .unwrap();

    assert_eq!(result.records_seen, 3);
    assert_eq!(result.events_written, 1);
    assert_eq!(result.events_skipped, 2);
    assert_eq!(result.skip_reasons.get("duplicate_event"), Some(&2));
}

#[test]
fn local_app_routes_emit_clean_readable_markdown() {
    let root = TempDir::new().unwrap();
    let output = root.path().join("output");
    fs::create_dir(&output).unwrap();

    let fixtures = [
        (
            "apple-mail-local",
            apple_mail_fixture(root.path()),
            [
                "Hello from Apple Mail.",
                "# Local Apple Mail fixture",
                "Direction: received",
            ]
            .as_slice(),
            ["[data-ogsc]", "<style>", "color: green"].as_slice(),
        ),
        (
            "apple-music-local-library",
            apple_music_local_fixture(root.path()),
            [
                "Unique tracks preserved: **1**",
                "Local audio files with embedded metadata where available",
                "file timestamps are metadata only",
            ]
            .as_slice(),
            ["Modified:", "date: \"2026-"].as_slice(),
        ),
        (
            "immich-local-library",
            immich_fixture(root.path()),
            [
                "Immich local file: `immich-original.png`",
                "Device: not available",
                "Source path remains local and is preserved only as provenance.",
            ]
            .as_slice(),
            ["network", "thumbnail"].as_slice(),
        ),
        (
            "apple-calendar-local",
            apple_calendar_fixture(root.path()),
            [
                "Calendar: Apple Calendar",
                "Summary: Coffee, planning",
                "Location: Kitchen table",
                "Attendees: mailto:friend@example.test",
            ]
            .as_slice(),
            ["BEGIN:VEVENT", "\\nKeep it offline."].as_slice(),
        ),
        (
            "apple-contacts-local",
            apple_contacts_fixture(root.path()),
            [
                "Contact: **Alex Example**",
                "Organization: Amara Local",
                "Birthday: 1980-01-02",
                "Addresses: Main Street 1, Munich, 80331, Germany",
                "Notes: private notes present: 1 note field(s); note bodies are not emitted by default.",
            ]
            .as_slice(),
            ["Private note that should not be emitted.", ";;Main Street"].as_slice(),
        ),
    ];

    for (adapter_id, source, expected, forbidden) in fixtures {
        let result = extract(
            ExtractionRequest {
                adapter_id: adapter_id.into(),
                source,
                destination: output.clone(),
            },
            &AtomicBool::new(false),
            &mut |_| {},
        )
        .unwrap_or_else(|error| panic!("{adapter_id} failed: {error}"));

        if adapter_id == "apple-music-local-library" {
            assert_eq!(result.quality.status, "review", "{adapter_id} quality");
            assert_eq!(result.quality.score_basis_points, 0, "{adapter_id} score");
        } else {
            assert_eq!(result.quality.status, "pass", "{adapter_id} quality");
            assert_eq!(
                result.quality.score_basis_points, 10_000,
                "{adapter_id} score"
            );
        }
        let manifest: serde_yaml::Value = serde_yaml::from_str(
            &fs::read_to_string(Path::new(&result.folder).join("manifest.yaml")).unwrap(),
        )
        .unwrap();
        assert_eq!(manifest["networkUsed"], false, "{adapter_id} network use");

        let markdown = if adapter_id == "apple-music-local-library" {
            collect_markdown(Path::new(&result.folder).to_path_buf())
        } else {
            collect_markdown(Path::new(&result.folder).join("events"))
        };
        for value in expected {
            assert!(
                markdown.contains(value),
                "{adapter_id} missing readable value {value:?}\n{markdown}"
            );
        }
        for value in forbidden {
            assert!(
                !markdown.contains(value),
                "{adapter_id} leaked forbidden value {value:?}\n{markdown}"
            );
        }
        if adapter_id == "apple-music-local-library" {
            let package = Path::new(&result.folder);
            let catalog = fs::read_to_string(package.join("library/a.md")).unwrap();
            let coverage = fs::read_to_string(package.join("APPLE-MUSIC-COVERAGE.md")).unwrap();
            assert!(catalog.contains("## Artist"));
            assert!(catalog.contains("### Album"));
            assert!(catalog.contains("Local Song"));
            assert!(coverage.contains("Filesystem creation/modification times"));
            assert_eq!(result.source_records_included, 1);
        }
    }
}

#[test]
fn apple_mail_exported_mbox_package_folder_emits_clean_markdown() {
    let root = TempDir::new().unwrap();
    let output = root.path().join("output");
    fs::create_dir(&output).unwrap();
    let source = apple_mail_fixture(root.path()).join("Inbox.mbox");

    let result = extract(
        ExtractionRequest {
            adapter_id: "apple-mail-local".into(),
            source,
            destination: output,
        },
        &AtomicBool::new(false),
        &mut |_| {},
    )
    .unwrap_or_else(|error| panic!("apple-mail-local failed: {error}"));

    assert_eq!(result.quality.status, "pass");
    assert_eq!(result.records_seen, 1);
    assert_eq!(result.events_written, 1);
    let package = Path::new(&result.folder);
    verify_open_package(package, "apple-mail-local");
    let markdown = collect_markdown(package.join("events"));
    assert!(markdown.contains("Hello from Apple Mail."));
    assert!(markdown.contains("# Local Apple Mail fixture"));
    assert!(!markdown.contains("[data-ogsc]"));
    assert!(!markdown.contains("<style>"));
}

#[test]
fn local_app_routes_report_unreadable_or_empty_real_stores() {
    let root = TempDir::new().unwrap();
    let output = root.path().join("output");
    fs::create_dir(&output).unwrap();

    for (adapter_id, expected) in [
        ("apple-mail-local", "No readable Apple Mail export"),
        (
            "apple-calendar-local",
            "No readable Apple Calendar .ics files",
        ),
    ] {
        let source = directory(root.path(), &format!("{adapter_id}-empty"));
        let error = extract(
            ExtractionRequest {
                adapter_id: adapter_id.into(),
                source,
                destination: output.clone(),
            },
            &AtomicBool::new(false),
            &mut |_| {},
        )
        .expect_err("empty local app source should not look successful");
        assert!(
            error.to_string().contains(expected),
            "{adapter_id} error was not user-actionable: {error}"
        );
    }
}

#[test]
fn apple_mail_rejects_raw_emlx_store_and_keeps_export_only_contract() {
    let root = TempDir::new().unwrap();
    let output = root.path().join("output");
    fs::create_dir(&output).unwrap();
    let source = directory(root.path(), "raw-mail-store");
    let inbox = directory(&source, "Inbox.mbox");
    fs::write(
        inbox.join("message.emlx"),
        b"42\nDate: Tue, 2 Jan 2024 10:00:00 +0000\r\nSubject: Raw emlx\r\n\r\nHi",
    )
    .unwrap();

    let error = extract(
        ExtractionRequest {
            adapter_id: "apple-mail-local".into(),
            source,
            destination: output,
        },
        &AtomicBool::new(false),
        &mut |_| {},
    )
    .expect_err("Apple Mail adapter should require exported .mbox packages");

    assert!(error.to_string().contains("No readable Apple Mail export"));
    assert!(error.to_string().contains("Mailbox › Export Mailbox"));
}

#[test]
fn rerun_is_idempotent_and_preserves_existing_events() {
    let root = TempDir::new().unwrap();
    let output = root.path().join("output");
    fs::create_dir(&output).unwrap();
    let source = youtube_fixture(root.path());
    let request = || ExtractionRequest {
        adapter_id: "youtube-takeout".into(),
        source: source.clone(),
        destination: output.clone(),
    };
    let first = extract(request(), &AtomicBool::new(false), &mut |_| {}).unwrap();
    let second = extract(request(), &AtomicBool::new(false), &mut |_| {}).unwrap();
    assert_eq!(first.events_written, 1);
    assert_eq!(second.events_written, 0);
    assert_eq!(second.events_skipped, 1);
}

#[test]
fn airbnb_export_accounts_for_every_category_and_all_reservation_statuses() {
    let root = TempDir::new().unwrap();
    let output = root.path().join("output");
    fs::create_dir(&output).unwrap();
    let result = extract(
        ExtractionRequest {
            adapter_id: "airbnb-json-takeout".into(),
            source: airbnb_fixture(root.path()),
            destination: output,
        },
        &AtomicBool::new(false),
        &mut |_| {},
    )
    .unwrap();

    assert_eq!(result.records_seen, 9);
    assert_eq!(result.events_written, 6);
    assert_eq!(result.events_skipped, 3);
    assert_eq!(
        result
            .skip_reasons
            .get("booking_session_supporting_metadata"),
        Some(&1)
    );
    assert_eq!(
        result
            .skip_reasons
            .get("not_timeline_material_activity_log"),
        Some(&2)
    );
    assert!(result.quality.accounting_balanced);
    assert!(result.quality.skip_reasons_balanced);
    let markdown = walkdir::WalkDir::new(Path::new(&result.folder).join("events"))
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.path().extension().is_some_and(|value| value == "md"))
        .map(|entry| fs::read_to_string(entry.path()).unwrap())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(markdown.contains("title: Airbnb stay - 1 Jun 2024 to 4 Jun 2024"));
    assert!(markdown.contains("type: message"));
    assert!(markdown.contains("A co-traveler joined the trip."));
    assert!(!markdown.contains("MessageContent{"));
    assert!(markdown.contains("Airbnb review you wrote"));
    assert!(markdown.contains("Airbnb review you received"));
    assert!(markdown.contains("title: Airbnb cancelled reservation - 1 Jul 2024 to 3 Jul 2024"));
    assert!(markdown.contains("type: reservation"));
    assert!(!markdown.contains("2024-08-01"));
    let coverage =
        fs::read_to_string(Path::new(&result.folder).join("AIRBNB-COVERAGE.md")).unwrap();
    assert!(coverage.contains("`reservations.json` | 3 | 2 | 1"));
    assert!(coverage.contains("`activity_log.json` | 2 | 0 | 2"));
    assert!(coverage.contains("not_timeline_material_activity_log"));
}

#[test]
fn banking_routes_read_only_their_declared_format() {
    let root = TempDir::new().unwrap();
    let source = banking_fixture(root.path());
    fs::write(
        source.join("transactions.csv"),
        "Date,Amount,Currency,Counterparty,Description\n2024-03-05,-9.50,EUR,CSV Only Cafe,Breakfast\n2024-03-06,1200.00,EUR,Employer,Salary\n",
    )
    .unwrap();
    let output = root.path().join("output");
    fs::create_dir(&output).unwrap();

    let result = extract(
        ExtractionRequest {
            adapter_id: "banking-csv".into(),
            source,
            destination: output,
        },
        &AtomicBool::new(false),
        &mut |_| {},
    )
    .unwrap();
    // Both source transactions become exact-date daily files; the month overview is secondary.
    assert_eq!(result.records_seen, 3);
    assert_eq!(result.source_records_included, 2);
    assert_eq!(result.events_written, 3);
    let markdown = walkdir::WalkDir::new(Path::new(&result.folder).join("events"))
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.path().extension().is_some_and(|value| value == "md"))
        .map(|entry| fs::read_to_string(entry.path()).unwrap())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(markdown.contains("CSV Only Cafe"));
    assert!(markdown.contains("Employer"));
    assert!(markdown.contains("1 banking movement was recorded for this day."));
    assert!(markdown.contains("type: daily_movements"));
    assert!(markdown.contains("date_precision: day"));
    assert!(markdown.contains("type: monthly_overview"));
    assert!(markdown.contains("date_precision: month"));
    assert!(markdown.contains("2 banking movements across 2 active days"));
    assert!(!markdown.contains("| Date | Amount |"));
    assert!(!markdown.contains("Corner Cafe"));
}

#[test]
fn banking_csv_repairs_two_digit_years_and_suppresses_atm_identities() {
    let root = TempDir::new().unwrap();
    let source = directory(root.path(), "banking-two-digit-year-source");
    fs::write(
        source.join("transactions.csv"),
        "Buchungstag;Betrag;Waehrung;Auftraggeber/Empfänger;Verwendungszweck\n04.03.24;-12,50;EUR;Corner Cafe;Lunch\n06.04.25;-42,00;EUR;Ga;GA NR123 BLZ456 0\n",
    )
    .unwrap();
    let output = root.path().join("output");
    fs::create_dir(&output).unwrap();

    let result = extract(
        ExtractionRequest {
            adapter_id: "banking-csv".into(),
            source,
            destination: output,
        },
        &AtomicBool::new(false),
        &mut |_| {},
    )
    .unwrap();

    assert_eq!(result.events_written, 4);
    assert_eq!(result.source_records_included, 2);
    assert_eq!(result.quality.implausible_date_warnings, 0);
    assert_eq!(result.quality.counterparty_warnings, 0);
    assert_eq!(result.quality.score_basis_points, 10_000);
    let event_root = Path::new(&result.folder).join("events");
    assert!(event_root.join("2024/03").is_dir());
    assert!(event_root.join("2025/04").is_dir());
    let markdown = walkdir::WalkDir::new(&event_root)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.path().extension().is_some_and(|value| value == "md"))
        .map(|entry| fs::read_to_string(entry.path()).unwrap())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(markdown.contains("2024-03-04"));
    assert!(markdown.contains("2025-04-06"));
    assert!(!markdown.contains("0024-03-04"));
    assert!(!markdown.contains("0025-04-06"));
    assert!(!markdown.contains("merchant-ga"));
    assert!(!markdown.contains("\n- ga\n"));
    assert!(markdown.contains("## Movement 1 · -42,00 EUR — Unspecified target"));
    assert!(markdown.contains("GA NR123 BLZ456 0"));
}

#[test]
fn banking_csv_recognizes_title_rows_tabs_and_separate_debit_credit_columns() {
    let root = TempDir::new().unwrap();
    let source = directory(root.path(), "banking-generic-source");
    fs::write(
        source.join("transactions.csv"),
        "Portable account export\nTransaction Date\tDebit Amount\tCredit Amount\tCurrency Code\tPayee\tAccount Name\tDetails\n2026-01-03\t12.40\t\tEUR\tCorner Cafe\tDaily account\tLunch\n2026-01-15\t\t2500.00\tEUR\tEmployer\tDaily account\tSalary\n",
    )
    .unwrap();
    let output = root.path().join("output");
    fs::create_dir(&output).unwrap();

    let result = extract(
        ExtractionRequest {
            adapter_id: "banking-csv".into(),
            source,
            destination: output,
        },
        &AtomicBool::new(false),
        &mut |_| {},
    )
    .unwrap();

    assert_eq!(result.records_seen, 3);
    assert_eq!(result.source_records_included, 2);
    assert_eq!(result.events_written, 3);
    assert_eq!(result.quality.status, "pass");
    let markdown = walkdir::WalkDir::new(Path::new(&result.folder).join("events"))
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.path().extension().is_some_and(|value| value == "md"))
        .map(|entry| fs::read_to_string(entry.path()).unwrap())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(markdown.contains("## Movement 1 · -12.40 EUR — Corner Cafe"));
    assert!(markdown.contains("- Account / source: Daily account"));
    assert!(markdown.contains("- Description: Lunch"));
    assert!(markdown.contains("## Movement 1 · 2500.00 EUR — Employer"));
    assert!(markdown.contains("- Description: Salary"));
    assert!(markdown.contains("2 banking movements across 2 active days"));
    assert!(!markdown.contains("|---"));
}

fn verify_open_package(root: &Path, adapter_id: &str) {
    let manifest: serde_yaml::Value =
        serde_yaml::from_str(&fs::read_to_string(root.join("manifest.yaml")).unwrap()).unwrap();
    assert_eq!(manifest["adapterId"], adapter_id);
    assert_eq!(manifest["networkUsed"], false);
    assert_eq!(manifest["quality"]["status"], "pass");
    assert_eq!(manifest["quality"]["scoreBasisPoints"], 10_000);
    assert!(root.join("README.md").is_file());
    assert!(root.join("quality-report.md").is_file());
    assert!(root.join("quality-report.json").is_file());
    let event = walkdir::WalkDir::new(root.join("events"))
        .into_iter()
        .filter_map(Result::ok)
        .find(|entry| entry.path().extension().is_some_and(|value| value == "md"))
        .expect("event Markdown missing");
    let markdown = fs::read_to_string(event.path()).unwrap();
    let yaml = markdown
        .strip_prefix("---\n")
        .and_then(|content| content.split_once("\n---\n").map(|parts| parts.0))
        .expect("frontmatter missing");
    let frontmatter: serde_yaml::Value = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(frontmatter["schema_version"], 1);
    assert!(frontmatter["id"]
        .as_str()
        .is_some_and(|value| !value.is_empty()));
    assert!(frontmatter["title"]
        .as_str()
        .is_some_and(|value| !value.is_empty()));
    assert!(frontmatter["provenance"]["raw_path"].as_str().is_some());
    assert!(frontmatter["provenance"]["raw_hash"]
        .as_str()
        .unwrap()
        .starts_with("sha256:"));
}

fn collect_markdown(root: PathBuf) -> String {
    walkdir::WalkDir::new(root)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.path().extension().is_some_and(|value| value == "md"))
        .map(|entry| fs::read_to_string(entry.path()).unwrap())
        .collect::<Vec<_>>()
        .join("\n")
}

fn photo_fixture(root: &Path) -> PathBuf {
    let source = directory(root, "photo-source");
    fs::write(source.join("tiny.png"), hex("89504e470d0a1a0a0000000d49484452000000010000000108060000001f15c4890000000d4944415408d763f8cfc0f01f00050001ff89993d1d0000000049454e44ae426082")).unwrap();
    source
}

fn immich_fixture(root: &Path) -> PathBuf {
    let source = directory(root, "immich-local-library");
    fs::write(source.join("immich-original.png"), hex("89504e470d0a1a0a0000000d49484452000000010000000108060000001f15c4890000000d4944415408d763f8cfc0f01f00050001ff89993d1d0000000049454e44ae426082")).unwrap();
    source
}

fn apple_mail_fixture(root: &Path) -> PathBuf {
    let source = directory(root, "apple-mail-local");
    let mailbox = source.join("Inbox.mbox");
    fs::create_dir(&mailbox).unwrap();
    fs::write(
        mailbox.join("mbox"),
        b"From sender@example.test Tue Jan 02 10:00:00 2024\nDate: Tue, 2 Jan 2024 10:00:00 +0000\r\nFrom: Sender <sender@example.test>\r\nTo: Receiver <receiver@example.test>\r\nSubject: Local Apple Mail fixture\r\nContent-Type: text/html; charset=utf-8\r\n\r\n<style>[data-ogsc] { color: green; }</style><p>Hello from Apple Mail.</p>\r\n",
    )
    .unwrap();
    source
}

fn apple_calendar_fixture(root: &Path) -> PathBuf {
    let source = directory(root, "apple-calendar-local");
    fs::write(
        source.join("home.ics"),
        "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nBEGIN:VEVENT\r\nUID:calendar-fixture-1\r\nDTSTART:20240721T103000Z\r\nDTEND:20240721T113000Z\r\nSUMMARY:Coffee\\, planning\r\nLOCATION:Kitchen table\r\nORGANIZER:mailto:me@example.test\r\nATTENDEE:mailto:friend@example.test\r\nDESCRIPTION:Talk through local-app intake\\nKeep it offline.\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n",
    )
    .unwrap();
    source
}

fn apple_contacts_fixture(root: &Path) -> PathBuf {
    let source = directory(root, "apple-contacts-local");
    fs::write(
        source.join("contacts.vcf"),
        "BEGIN:VCARD\r\nVERSION:3.0\r\nUID:contact-fixture-1\r\nFN:Alex Example\r\nN:Example;Alex;;;\r\nORG:Amara Local\r\nTITLE:Tester\r\nBDAY:19800102\r\nEMAIL:alex@example.test\r\nTEL:+49123456789\r\nADR:;;Main Street 1;Munich;;80331;Germany\r\nNOTE:Private note that should not be emitted.\r\nEND:VCARD\r\n",
    )
    .unwrap();
    source
}

fn apple_music_local_fixture(root: &Path) -> PathBuf {
    let source = directory(root, "apple-music-local");
    let album = source.join("Artist/Album");
    fs::create_dir_all(&album).unwrap();
    fs::write(album.join("Local Song.m4a"), b"synthetic local audio").unwrap();
    source
}

fn banking_fixture(root: &Path) -> PathBuf {
    let source = directory(root, "banking-source");
    write_zip(
        &source.join("movements.xlsx"),
        &[
            (
                "[Content_Types].xml",
                r#"<?xml version="1.0"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/><Override PartName="/xl/worksheets/sheet1.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/></Types>"#,
            ),
            (
                "_rels/.rels",
                r#"<?xml version="1.0"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#,
            ),
            (
                "xl/workbook.xml",
                r#"<?xml version="1.0"?><workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><sheets><sheet name="Readable_Ledger" sheetId="1" r:id="rId1"/></sheets></workbook>"#,
            ),
            (
                "xl/_rels/workbook.xml.rels",
                r#"<?xml version="1.0"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/></Relationships>"#,
            ),
            (
                "xl/worksheets/sheet1.xml",
                r#"<?xml version="1.0"?><worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheetData><row r="1"><c t="inlineStr"><is><t>Date</t></is></c><c t="inlineStr"><is><t>Amount</t></is></c><c t="inlineStr"><is><t>Currency</t></is></c><c t="inlineStr"><is><t>Counterparty</t></is></c><c t="inlineStr"><is><t>Description</t></is></c></row><row r="2"><c t="inlineStr"><is><t>2024-03-04</t></is></c><c t="inlineStr"><is><t>-12.50</t></is></c><c t="inlineStr"><is><t>EUR</t></is></c><c t="inlineStr"><is><t>Corner Cafe</t></is></c><c t="inlineStr"><is><t>Lunch</t></is></c></row></sheetData></worksheet>"#,
            ),
        ],
    );
    source
}

fn banking_csv_fixture(root: &Path) -> PathBuf {
    let source = directory(root, "banking-csv-source");
    fs::write(
        source.join("transactions.csv"),
        "Buchungstag;Betrag;Waehrung;Auftraggeber/Empfänger;Verwendungszweck\n04.03.2024;-12,50;EUR;Corner Cafe;Lunch\n",
    )
    .unwrap();
    source
}

fn airbnb_fixture(root: &Path) -> PathBuf {
    let source = directory(root, "airbnb-source");
    fs::create_dir(source.join("json")).unwrap();
    fs::write(source.join("json/reservations.json"), r#"[{"reservations":[{"startDate":"2024-06-01","nights":3,"hostingUrl":"https://airbnb.example/rooms/1","numberOfGuests":2,"status":"accepted"},{"startDate":"2024-07-01","nights":2,"status":"cancelled"}],"bookingSessions":[{"homeInquiries":[{"startDate":"2024-08-01","nights":5}]}]}]"#).unwrap();
    fs::write(source.join("json/messages.json"), r#"[{"messageThreads":[{"id":42,"messagesAndContents":[{"message":{"accountType":"guest","createdAt":"2024-05-01T12:30:00Z"},"messageContent":{"textContent":{"body":"Looking forward to the stay."}}},{"message":{"accountType":"service","createdAt":"2024-05-01T12:31:00Z"},"messageContent":{"messageContentV2":{"content":"MessageContent{value=StaticBulletinContent{body=StandardPhrase{phraseKey=messaging.default_cotraveler_add, fallback=A co-traveler joined the trip., interpolationParameters={smart_name=Guest}}, contentSubtype=STAYS_COTRAVELER_JOINED_THREAD}}"}}}]}]}]"#).unwrap();
    fs::write(source.join("json/reviews.json"), r#"[{"reviewsProvided":[{"review":{"submittedAt":"2024-06-05T10:00:00Z","comment":"A welcoming home.","rating":5,"revieweeId":10}}],"reviewsReceived":[{"review":{"submittedAt":"2024-06-06T11:00:00Z","comment":"A considerate guest.","rating":5,"reviewerId":20}}]}]"#).unwrap();
    fs::write(source.join("json/activity_log.json"), r#"[{"sessionActivities":[{"createdAt":"2024-01-01T10:00:00Z","activityType":"login"},{"createdAt":"2024-01-02T11:00:00Z","activityType":"search"}]}]"#).unwrap();
    source
}

fn gmail_fixture(root: &Path) -> PathBuf {
    let source = directory(root, "gmail-source");
    write_zip(&source.join("takeout.zip"), &[("Takeout/Gmail/All mail.mbox", "From sender@example.com Sat Jan  1 00:00:00 2022\nDate: Sat, 1 Jan 2022 10:20:00 +0000\nFrom: Alice <alice@example.com>\nTo: me@example.com\nSubject: Winter note\nContent-Type: text/plain; charset=utf-8\n\nHello from the archive.\n")]);
    source
}

fn diary_fixture(root: &Path) -> PathBuf {
    let source = directory(root, "diary-source");
    write_zip(
        &source.join("diary.docx"),
        &[(
            "word/document.xml",
            r#"<?xml version="1.0"?><w:document xmlns:w="urn:test"><w:body><w:p><w:pPr><w:pStyle w:val="berschrift1"/></w:pPr><w:r><w:t>2024</w:t></w:r></w:p><w:p><w:pPr><w:pStyle w:val="berschrift3"/></w:pPr><w:r><w:t>März</w:t></w:r></w:p><w:p><w:pPr><w:pStyle w:val="berschrift4"/></w:pPr><w:r><w:t>First day</w:t></w:r></w:p><w:p><w:r><w:t>A verbatim diary paragraph.</w:t></w:r></w:p></w:body></w:document>"#,
        )],
    );
    source
}

fn youtube_fixture(root: &Path) -> PathBuf {
    let source = directory(root, "youtube-source");
    write_zip(&source.join("takeout.zip"), &[("Takeout/YouTube and YouTube Music/subscriptions/subscriptions.csv", "Channel Id,Channel Url,Channel Title\nabc,https://youtube.example/abc,Archive Channel\n")]);
    source
}

fn google_photos_fixture(root: &Path) -> PathBuf {
    let source = directory(root, "google-photos-source");
    write_zip(
        &source.join("takeout.zip"),
        &[(
            "Takeout/Google Photos/Photos from 2022/image.jpg.supplemental-metadata.json",
            r#"{"title":"image.jpg","description":"A day outside","photoTakenTime":{"timestamp":"1655297580"},"geoData":{"latitude":48.1,"longitude":11.5}}"#,
        )],
    );
    source
}

fn kmz_fixture(root: &Path) -> PathBuf {
    let source = directory(root, "kmz-source");
    let kmz = zip_bytes(&[(
        "doc.kml",
        r#"<?xml version="1.0"?><kml xmlns="http://www.opengis.net/kml/2.2"><Document><Placemark><name>Open Place</name><Point><coordinates>11.5,48.1,0</coordinates></Point></Placemark><Placemark><name>Open Place</name><Point><coordinates>11.5,48.1,0</coordinates></Point></Placemark><Placemark><name>No Coordinates</name></Placemark></Document></kml>"#,
    )]);
    let file = fs::File::create(source.join("takeout.zip")).unwrap();
    let mut archive = ZipWriter::new(file);
    archive
        .start_file("Takeout/Drive/places.kmz", SimpleFileOptions::default())
        .unwrap();
    archive.write_all(&kmz).unwrap();
    archive.finish().unwrap();
    source
}

fn standalone_kmz_fixture(root: &Path) -> PathBuf {
    let source = root.join("standalone-map.kmz");
    fs::write(
        &source,
        zip_bytes(&[(
            "doc.kml",
            r#"<?xml version="1.0"?><kml xmlns="http://www.opengis.net/kml/2.2"><Document><Placemark><name>Standalone Place</name><Point><coordinates>11.5,48.1,0</coordinates></Point></Placemark></Document></kml>"#,
        )]),
    )
    .unwrap();
    source
}

fn maps_csv_zip_fixture(root: &Path) -> PathBuf {
    let source = root.join("maps-csv.zip");
    write_zip(
        &source,
        &[
            (
                "Google Maps/ALL_Locations.csv",
                "Title,Comment,Google Maps URL,Note,Latitude,Longitude\nOpen Library,,https://maps.example/library,Quiet place,48.1,11.5\n",
            ),
            (
                "Google Maps/Munich.csv",
                "Title,Comment,Google Maps URL,Note,Latitude,Longitude\nOpen Library,Favorite,https://maps.example/library,Quiet place,48.1,11.5\n",
            ),
        ],
    );
    source
}

fn maps_geojson_fixture(root: &Path) -> PathBuf {
    let source = directory(root, "maps-geojson");
    let folder = source.join("Takeout/Maps (Meine Orte)");
    fs::create_dir_all(&folder).unwrap();
    fs::write(
        folder.join("Gespeicherte Orte.json"),
        r#"{"type":"FeatureCollection","features":[{"geometry":{"coordinates":[11.5,48.1],"type":"Point"},"properties":{"Google Maps URL":"http://maps.google.com/?cid=123456","Location":{"Address":"42 Example Street, Example City","Business Name":"Example Campground","Country Code":"AT","Geo Coordinates":{"Latitude":"48.1","Longitude":"11.5"}},"Published":"2023-04-12T12:45:44Z","Title":"Example Campground","Updated":"2023-04-12T12:45:46Z"},"type":"Feature"}]}"#,
    )
    .unwrap();
    source
}

fn apple_music_fixture(root: &Path) -> PathBuf {
    let source = directory(root, "apple-music-source");
    fs::create_dir(source.join("raw")).unwrap();
    fs::write(source.join("raw/library_tracks.tsv"), "Name\tArtist\tAlbum\tGenre\tDateAdded\tPlayCount\nOpen Song\tLocal Artist\tOpen Album\tRock\t2024-01-02 12:30:00\t7\n").unwrap();
    source
}

fn directory(root: &Path, name: &str) -> PathBuf {
    let path = root.join(name);
    fs::create_dir(&path).unwrap();
    path
}

fn write_zip(path: &Path, entries: &[(&str, &str)]) {
    let file = fs::File::create(path).unwrap();
    let mut zip = ZipWriter::new(file);
    for (name, body) in entries {
        zip.start_file(*name, SimpleFileOptions::default()).unwrap();
        zip.write_all(body.as_bytes()).unwrap();
    }
    zip.finish().unwrap();
}

fn zip_bytes(entries: &[(&str, &str)]) -> Vec<u8> {
    let mut cursor = Cursor::new(Vec::new());
    {
        let mut zip = ZipWriter::new(&mut cursor);
        for (name, body) in entries {
            zip.start_file(*name, SimpleFileOptions::default()).unwrap();
            zip.write_all(body.as_bytes()).unwrap();
        }
        zip.finish().unwrap();
    }
    cursor.into_inner()
}

fn hex(value: &str) -> Vec<u8> {
    (0..value.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(&value[index..index + 2], 16).unwrap())
        .collect()
}

#[test]
fn reliability_bank_csv_uses_header_delimiter_and_ignores_other_csv_in_batch() {
    let root = TempDir::new().unwrap();
    let source = directory(root.path(), "mixed");
    fs::write(source.join("ledger.csv"), "Synthetic export title\nDate;Amount;Currency;Description\n2024-09-07;-12.50;EUR;\"one,two,three,four,five,six\"\n").unwrap();
    fs::write(
        source.join("places.csv"),
        "Title,URL,Note\nSynthetic place,https://example.invalid/place,Not a transaction\n",
    )
    .unwrap();
    let output = directory(root.path(), "out");
    let result = extract(
        ExtractionRequest {
            adapter_id: "banking-csv".into(),
            source,
            destination: output,
        },
        &AtomicBool::new(false),
        &mut |_| {},
    )
    .unwrap();
    assert_eq!(result.source_records_included, 1);
    assert!(result.events_written > 0);
    assert!(collect_markdown(Path::new(&result.folder).join("events"))
        .contains("one,two,three,four,five,six"));
}

#[test]
fn reliability_cancelled_rerun_preserves_complete_package_byte_for_byte() {
    use std::sync::atomic::Ordering;
    let root = TempDir::new().unwrap();
    let source = diary_fixture(root.path());
    let output = directory(root.path(), "out");
    let request = ExtractionRequest {
        adapter_id: "diary-docx".into(),
        source: source.clone(),
        destination: output,
    };
    let first = extract(request.clone(), &AtomicBool::new(false), &mut |_| {}).unwrap();
    let before = package_bytes(Path::new(&first.folder));
    let cancel = AtomicBool::new(false);
    let second = extract(request, &cancel, &mut |progress| {
        if progress.records_seen > 0 {
            cancel.store(true, Ordering::Relaxed);
        }
    })
    .unwrap();
    assert!(second.cancelled);
    assert_eq!(package_bytes(Path::new(&first.folder)), before);
}

#[test]
fn reliability_late_parse_failure_does_not_publish_partial_diary() {
    let root = TempDir::new().unwrap();
    let source = diary_fixture(root.path());
    let output = directory(root.path(), "out");
    let mut corrupted = false;
    let broken = source.join("later.docx");
    fs::copy(source.join("diary.docx"), &broken).unwrap();
    let result = extract(
        ExtractionRequest {
            adapter_id: "diary-docx".into(),
            source,
            destination: output.clone(),
        },
        &AtomicBool::new(false),
        &mut |progress| {
            if progress.events_written > 0 && !corrupted {
                // Both input paths are inventoried before callbacks; break the unread one.
                fs::write(&broken, b"not a DOCX").unwrap();
                corrupted = true;
            }
        },
    );
    assert!(result.is_err());
    assert!(
        package_bytes(&output).is_empty(),
        "failed extraction published files"
    );
}

fn package_bytes(root: &Path) -> std::collections::BTreeMap<String, Vec<u8>> {
    walkdir::WalkDir::new(root)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .map(|entry| {
            (
                entry
                    .path()
                    .strip_prefix(root)
                    .unwrap()
                    .to_string_lossy()
                    .into_owned(),
                fs::read(entry.path()).unwrap(),
            )
        })
        .collect()
}

#[test]
fn reliability_source_output_overlap_is_rejected_without_changing_originals() {
    let root = TempDir::new().unwrap();
    let source = diary_fixture(root.path());
    let before = package_bytes(&source);
    let result = extract(
        ExtractionRequest {
            adapter_id: "diary-docx".into(),
            source: source.clone(),
            destination: source.clone(),
        },
        &AtomicBool::new(false),
        &mut |_| {},
    );
    assert!(result.is_err());
    assert_eq!(package_bytes(&source), before);
}

#[test]
fn reliability_maps_geojson_extension_is_included_and_reruns_without_duplicates() {
    let root = TempDir::new().unwrap();
    let source = maps_geojson_fixture(root.path());
    let existing = source.join("Takeout/Maps (Meine Orte)/Gespeicherte Orte.json");
    let geojson = source.join("Saved Places.geojson");
    fs::rename(existing, &geojson).unwrap();
    let original = fs::read(&geojson).unwrap();
    let output = directory(root.path(), "out");
    let request = ExtractionRequest {
        adapter_id: "google-maps".into(),
        source: source.clone(),
        destination: output.clone(),
    };
    let first = extract(request.clone(), &AtomicBool::new(false), &mut |_| {}).unwrap();
    assert_eq!(first.events_written, 1);
    let before = package_bytes(Path::new(&first.folder).join("events").as_path());
    let second = extract(request, &AtomicBool::new(false), &mut |_| {}).unwrap();
    assert_eq!(second.events_written, 0);
    assert_eq!(second.events_skipped, 1);
    assert_eq!(
        package_bytes(Path::new(&second.folder).join("events").as_path()),
        before
    );
    assert_eq!(fs::read(&geojson).unwrap(), original);
    let archive = source.join("places.zip");
    write_zip(
        &archive,
        &[(
            "Saved Places.geojson",
            std::str::from_utf8(&original).unwrap(),
        )],
    );
    let archived = extract(
        ExtractionRequest {
            adapter_id: "google-maps".into(),
            source: archive,
            destination: output,
        },
        &AtomicBool::new(false),
        &mut |_| {},
    )
    .unwrap();
    assert_eq!(archived.events_written, 1);
}

#[test]
fn photo_without_capture_metadata_stays_undated() {
    let root = TempDir::new().unwrap();
    fs::create_dir(root.path().join("output")).unwrap();
    let source = photo_fixture(root.path());
    fs::write(source.join("captured.png"), hex("89504e470d0a1a0a0000000d49484452000000010000000108060000001f15c489000000406558496649492a0008000000010069870400010000001a00000000000000010003900200140000002c00000000000000323032343a30393a30372031343a32353a3030004b2321610000000d4944415408d763f8cfc0f01f00050001ff89993d1d0000000049454e44ae426082")).unwrap();
    let result = extract(
        ExtractionRequest {
            adapter_id: "photo-media-folder".into(),
            source,
            destination: root.path().join("output"),
        },
        &AtomicBool::new(false),
        &mut |_| {},
    )
    .unwrap();
    let markdown = collect_markdown(Path::new(&result.folder).join("events"));
    assert!(markdown.contains("date: \"0000-00-00\""), "{markdown}");
    assert!(markdown.contains("Capture date: not included"));
    assert!(markdown.contains("date: \"2024-09-07\""), "{markdown}");
    assert!(markdown.contains("time: \"14:25\""), "{markdown}");
    assert_eq!(result.events_written, 2);
}

#[test]
fn calendar_preserves_zone_and_exclusive_all_day_end() {
    let root = TempDir::new().unwrap();
    fs::create_dir(root.path().join("output")).unwrap();
    let source = directory(root.path(), "calendar-variants");
    fs::write(source.join("events.ics"), "BEGIN:VCALENDAR\nBEGIN:VEVENT\nUID:all-day\nDTSTART;VALUE=DATE:20240909\nDTEND;VALUE=DATE:20240910\nSUMMARY:One day\nEND:VEVENT\nBEGIN:VEVENT\nUID:zoned\nDTSTART;TZID=Europe/Berlin:20240908T093000\nDTEND;TZID=Europe/Berlin:20240908T103000\nSUMMARY:Berlin time\nEND:VEVENT\nEND:VCALENDAR\n").unwrap();
    let result = extract(
        ExtractionRequest {
            adapter_id: "apple-calendar-local".into(),
            source,
            destination: root.path().join("output"),
        },
        &AtomicBool::new(false),
        &mut |_| {},
    )
    .unwrap();
    let markdown = collect_markdown(Path::new(&result.folder).join("events"));
    assert!(!markdown.contains("end_date: \"2024-09-10\""), "{markdown}");
    assert!(markdown.contains("2024-09-08 09:30 (Europe/Berlin)"));
    assert!(markdown.contains("2024-09-10 (all-day date; exclusive end)"));
}

#[test]
fn calendar_recurrence_evidence_and_explicit_occurrences_survive_export() {
    let root = TempDir::new().unwrap();
    let source = directory(root.path(), "recurring-calendar");
    fs::write(source.join("recurring.ics"), "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nBEGIN:VEVENT\r\nUID:synthetic-series\r\nDTSTART;TZID=Europe/Berlin:20240205T100000\r\nDTEND;TZID=Europe/Berlin:20240205T110000\r\nRRULE:FREQ=WEEKLY;COUNT=3\r\nEXDATE;TZID=Europe/Berlin:20240212T100000\r\nRDATE;TZID=Europe/Berlin:20240220T100000\r\nSUMMARY:Synthetic weekly series\r\nEND:VEVENT\r\nBEGIN:VEVENT\r\nUID:synthetic-series\r\nRECURRENCE-ID;TZID=Europe/Berlin:20240219T100000\r\nDTSTART;TZID=Europe/Berlin:20240219T140000\r\nDTEND;TZID=Europe/Berlin:20240219T150000\r\nSUMMARY:Synthetic moved occurrence\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n").unwrap();
    let request = ExtractionRequest {
        adapter_id: "apple-calendar-local".into(),
        source,
        destination: directory(root.path(), "out"),
    };
    let result = extract(request.clone(), &AtomicBool::new(false), &mut |_| {}).unwrap();
    assert_eq!(
        result.events_written, 2,
        "series and explicit occurrence must remain distinct"
    );
    let text = collect_markdown(Path::new(&result.folder).join("events"));
    for evidence in [
        "RRULE:FREQ=WEEKLY;COUNT=3",
        "EXDATE;TZID=Europe/Berlin:20240212T100000",
        "RDATE;TZID=Europe/Berlin:20240220T100000",
        "RECURRENCE-ID;TZID=Europe/Berlin:20240219T100000",
        "Synthetic weekly series",
        "Synthetic moved occurrence",
        "not expanded",
    ] {
        assert!(
            text.contains(evidence),
            "missing source evidence: {evidence}"
        );
    }
    let rerun = extract(request, &AtomicBool::new(false), &mut |_| {}).unwrap();
    assert_eq!(rerun.events_written, 0);
    assert_eq!(
        collect_markdown(Path::new(&rerun.folder).join("events")),
        text
    );
}
