// SPDX-License-Identifier: AGPL-3.0-or-later

use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    fs::File,
    io::Read,
    path::{Path, PathBuf},
};
use walkdir::WalkDir;
use zip::ZipArchive;

const CATALOG: &str = include_str!("../../../../adapters/catalog.json");
const MAX_INSPECTED_PATHS: usize = 100_000;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AdapterCatalog {
    catalog_version: u32,
    output_contract: String,
    adapters: Vec<AdapterDefinition>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AdapterDefinition {
    id: String,
    label: String,
    provider: String,
    module_id: String,
    stage: String,
    format: String,
    readiness: String,
    #[serde(default)]
    signatures: Signatures,
    command_template: Option<String>,
    next_step: String,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Signatures {
    #[serde(default)]
    name_contains: Vec<String>,
    #[serde(default)]
    any_paths: Vec<String>,
    #[serde(default)]
    all_paths: Vec<String>,
    #[serde(default)]
    any_extensions: Vec<String>,
    #[serde(default)]
    archive_paths: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DatasetInspection {
    source_path: String,
    file_count: usize,
    inspected_path_count: usize,
    truncated: bool,
    catalog_version: u32,
    output_contract: String,
    detections: Vec<DatasetDetection>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DatasetDetection {
    adapter_id: String,
    label: String,
    provider: String,
    module_id: String,
    stage: String,
    format: String,
    readiness: String,
    evidence: Vec<String>,
    command_preview: Option<String>,
    next_step: String,
}

struct Inventory {
    source: PathBuf,
    paths: Vec<String>,
    archive_paths: Vec<String>,
    extensions: BTreeSet<String>,
    file_count: usize,
    truncated: bool,
}

struct RankedDetection {
    catalog_index: usize,
    specificity: u32,
    detection: DatasetDetection,
}

pub fn inspect(source: PathBuf) -> Result<DatasetInspection, String> {
    if !source.exists() {
        return Err(format!("{} no longer exists", source.display()));
    }
    let catalog: AdapterCatalog = serde_json::from_str(CATALOG)
        .map_err(|error| format!("invalid adapter catalog: {error}"))?;
    let inventory = inventory(source)?;
    let mut ranked = catalog
        .adapters
        .into_iter()
        .enumerate()
        .filter_map(|(catalog_index, adapter)| {
            detect(&inventory, adapter).map(|(detection, specificity)| RankedDetection {
                catalog_index,
                specificity,
                detection,
            })
        })
        .collect::<Vec<_>>();
    ranked.sort_by(|left, right| {
        right
            .specificity
            .cmp(&left.specificity)
            .then_with(|| left.catalog_index.cmp(&right.catalog_index))
    });
    let detections = ranked.into_iter().map(|item| item.detection).collect();
    Ok(DatasetInspection {
        source_path: inventory.source.to_string_lossy().into_owned(),
        file_count: inventory.file_count,
        inspected_path_count: inventory.paths.len() + inventory.archive_paths.len(),
        truncated: inventory.truncated,
        catalog_version: catalog.catalog_version,
        output_contract: catalog.output_contract,
        detections,
    })
}

fn inventory(source: PathBuf) -> Result<Inventory, String> {
    let mut paths = Vec::new();
    let mut archive_paths = Vec::new();
    let mut extensions = BTreeSet::new();
    let mut file_count = 0;
    let mut truncated = false;

    if source.is_file() {
        record_file(
            &source,
            source
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or_default(),
            &mut paths,
            &mut archive_paths,
            &mut extensions,
        );
        file_count = 1;
    } else {
        for entry in WalkDir::new(&source).follow_links(false) {
            let entry = match entry {
                Ok(value) => value,
                Err(_) => continue,
            };
            if paths.len() >= MAX_INSPECTED_PATHS {
                truncated = true;
                break;
            }
            let relative = entry
                .path()
                .strip_prefix(&source)
                .unwrap_or(entry.path())
                .to_string_lossy()
                .replace('\\', "/");
            if relative.is_empty() || apple_sidecar(&relative) {
                continue;
            }
            if entry.file_type().is_dir() {
                paths.push(format!("{relative}/"));
            } else if entry.file_type().is_file() {
                file_count += 1;
                record_file(
                    entry.path(),
                    &relative,
                    &mut paths,
                    &mut archive_paths,
                    &mut extensions,
                );
            }
        }
    }
    Ok(Inventory {
        source,
        paths,
        archive_paths,
        extensions,
        file_count,
        truncated,
    })
}

fn record_file(
    path: &Path,
    relative: &str,
    paths: &mut Vec<String>,
    archive_paths: &mut Vec<String>,
    extensions: &mut BTreeSet<String>,
) {
    paths.push(relative.to_owned());
    if let Some(extension) = path.extension().and_then(|value| value.to_str()) {
        let extension = extension.to_ascii_lowercase();
        extensions.insert(extension.clone());
        if extension == "zip" {
            inspect_zip(path, archive_paths);
        }
    }
    if path.file_name().and_then(|value| value.to_str()) == Some("mbox")
        && path
            .parent()
            .and_then(|parent| parent.extension())
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("mbox"))
    {
        extensions.insert("apple-mail-exported-mbox".into());
    }
}

fn inspect_zip(path: &Path, archive_paths: &mut Vec<String>) {
    let Ok(file) = File::open(path) else {
        return;
    };
    let Ok(mut archive) = ZipArchive::new(file) else {
        return;
    };
    for index in 0..archive.len().min(MAX_INSPECTED_PATHS) {
        let Ok(entry) = archive.by_index(index) else {
            continue;
        };
        let Ok(entry_name) = entry.name() else {
            continue;
        };
        let name = entry_name.replace('\\', "/");
        if !apple_sidecar(&name) {
            archive_paths.push(name);
        }
    }
}

fn detect(inventory: &Inventory, adapter: AdapterDefinition) -> Option<(DatasetDetection, u32)> {
    let source_name = inventory
        .source
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let mut adapter = adapter;
    if adapter.id == "diary-docx"
        && inventory
            .paths
            .iter()
            .filter(|path| path.to_ascii_lowercase().ends_with(".docx"))
            .take(50)
            .any(|relative| {
                let path = if inventory.source.is_file() {
                    inventory.source.clone()
                } else {
                    inventory.source.join(relative)
                };
                amara_unfold_adapters::recognizes_diary_docx(&path)
            })
    {
        adapter.signatures.any_paths.clear();
    }
    let signatures = &adapter.signatures;
    let mut evidence = Vec::new();
    let mut specificity = 0;
    let mut detected_format = adapter.format.clone();

    if adapter.id == "google-maps" {
        let recognition = google_maps_recognition(inventory)?;
        evidence = recognition.evidence;
        detected_format = recognition.format;
        specificity = recognition.specificity;
    } else if adapter.id == "banking-csv" {
        let matched = banking_csv_header_evidence(inventory)?;
        evidence.push(format!("recognized transaction columns in {matched}"));
        specificity += 700;
    } else if !signatures.name_contains.is_empty() {
        let token = signatures
            .name_contains
            .iter()
            .find(|token| source_name.contains(&token.to_ascii_lowercase()))?;
        evidence.push(format!("source name contains ‘{token}’"));
        specificity += 100;
    }
    for required in &signatures.all_paths {
        let matched = inventory
            .paths
            .iter()
            .find(|path| path_match(path, required))?;
        evidence.push(format!("contains {matched}"));
        specificity += 1_000;
    }
    if !signatures.any_paths.is_empty() {
        let matched = inventory.paths.iter().find(|path| {
            signatures
                .any_paths
                .iter()
                .any(|signature| path_match(path, signature))
        })?;
        evidence.push(format!("contains {matched}"));
        specificity += 500;
    }
    if !signatures.any_extensions.is_empty() {
        let matched = signatures.any_extensions.iter().find(|extension| {
            inventory
                .extensions
                .contains(&extension.to_ascii_lowercase())
        })?;
        evidence.push(extension_evidence(matched));
        specificity += 10;
    }
    if !signatures.archive_paths.is_empty() {
        let matched = inventory.archive_paths.iter().find(|path| {
            signatures
                .archive_paths
                .iter()
                .any(|signature| path_match(path, signature))
        })?;
        evidence.push(format!("archive contains {matched}"));
        specificity += 800;
    }
    if evidence.is_empty() {
        return None;
    }
    let command_preview = adapter.command_template.map(|template| {
        template.replace(
            "{source}",
            &format!("\"{}\"", inventory.source.to_string_lossy()),
        )
    });
    Some((
        DatasetDetection {
            adapter_id: adapter.id,
            label: adapter.label,
            provider: adapter.provider,
            module_id: adapter.module_id,
            stage: adapter.stage,
            format: detected_format,
            readiness: adapter.readiness,
            evidence,
            command_preview,
            next_step: adapter.next_step,
        },
        specificity,
    ))
}

struct GoogleMapsRecognition {
    evidence: Vec<String>,
    format: String,
    specificity: u32,
}

fn google_maps_recognition(inventory: &Inventory) -> Option<GoogleMapsRecognition> {
    let loose_kmz = inventory
        .paths
        .iter()
        .find(|path| path.to_ascii_lowercase().ends_with(".kmz"));
    let archived_kmz = inventory
        .archive_paths
        .iter()
        .find(|path| path.to_ascii_lowercase().ends_with(".kmz"));
    let loose_csv = inventory
        .paths
        .iter()
        .find(|path| is_all_locations_csv(path));
    let archived_csv = inventory
        .archive_paths
        .iter()
        .find(|path| is_all_locations_csv(path));
    let loose_json = inventory.paths.iter().find(|path| is_maps_json(path));
    let archived_json = inventory
        .archive_paths
        .iter()
        .find(|path| is_maps_json(path));

    let mut evidence = Vec::new();
    let mut formats = Vec::new();
    if let Some(path) = loose_kmz {
        evidence.push(format!("contains {path}"));
        formats.push("KMZ map file(s)");
    }
    if let Some(path) = archived_kmz {
        evidence.push(format!("archive contains {path}"));
        formats.push("KMZ inside Google Takeout ZIP");
    }
    if let Some(path) = loose_csv {
        evidence.push(format!("contains {path}"));
        formats.push("saved-place CSV collection");
    }
    if let Some(path) = archived_csv {
        evidence.push(format!("archive contains {path}"));
        formats.push("saved-place CSV inside ZIP");
    }
    if let Some(path) = loose_json {
        evidence.push(format!("contains {path}"));
        formats.push(maps_json_format(path));
    }
    if let Some(path) = archived_json {
        evidence.push(format!("archive contains {path}"));
        formats.push(maps_json_format(path));
    }
    if evidence.is_empty() {
        return None;
    }
    formats.sort_unstable();
    formats.dedup();
    Some(GoogleMapsRecognition {
        specificity: 1_200 + evidence.len() as u32 * 100,
        evidence,
        format: formats.join(" + "),
    })
}

fn is_all_locations_csv(path: &str) -> bool {
    path.rsplit('/')
        .next()
        .is_some_and(|name| name.eq_ignore_ascii_case("ALL_Locations.csv"))
}

fn is_maps_json(path: &str) -> bool {
    amara_unfold_adapters::recognizes_maps_json_path(path)
}

fn maps_json_format(path: &str) -> &'static str {
    if path.to_ascii_lowercase().ends_with("all-pois.json") {
        "normalized saved-place JSON"
    } else {
        "Google Maps saved-places GeoJSON"
    }
}

fn banking_csv_header_evidence(inventory: &Inventory) -> Option<String> {
    let candidates = inventory
        .paths
        .iter()
        .filter(|path| path.to_ascii_lowercase().ends_with(".csv"))
        .take(50);
    for relative in candidates {
        let path = if inventory.source.is_file() {
            inventory.source.clone()
        } else {
            inventory.source.join(relative)
        };
        let mut bytes = Vec::new();
        let Ok(file) = File::open(&path) else {
            continue;
        };
        if file.take(256 * 1024).read_to_end(&mut bytes).is_err() {
            continue;
        }
        if amara_unfold_adapters::recognizes_banking_csv(&String::from_utf8_lossy(&bytes)) {
            return Some(relative.to_owned());
        }
    }
    None
}

fn extension_evidence(extension: &str) -> String {
    match extension {
        "apple-mail-exported-mbox" => "contains Apple Mail .mbox package".into(),
        _ => format!("contains .{extension} files"),
    }
}

fn path_match(actual: &str, signature: &str) -> bool {
    let actual = actual.to_ascii_lowercase();
    let signature = signature.to_ascii_lowercase();
    actual == signature || actual.ends_with(&signature) || actual.contains(&signature)
}

fn apple_sidecar(path: &str) -> bool {
    path.split('/').any(|part| part.starts_with("._")) || path.ends_with(".DS_Store")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use zip::{write::SimpleFileOptions, ZipWriter};

    #[test]
    fn saved_places_geojson_extension_is_recognized_loose_and_archived() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("Saved Places.geojson");
        let body = br#"{"type":"FeatureCollection","features":[]}"#;
        std::fs::write(&path, body).unwrap();
        assert!(inspect(path)
            .unwrap()
            .detections
            .iter()
            .any(|item| item.adapter_id == "google-maps"));
        let path = directory.path().join("maps.zip");
        let mut archive = ZipWriter::new(std::fs::File::create(&path).unwrap());
        archive
            .start_file("Saved Places.geojson", SimpleFileOptions::default())
            .unwrap();
        archive.write_all(body).unwrap();
        archive.finish().unwrap();
        assert!(inspect(path)
            .unwrap()
            .detections
            .iter()
            .any(|item| item.adapter_id == "google-maps"));
    }

    #[test]
    fn diary_content_is_recognized_with_arbitrary_filename() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("memories.docx");
        let file = std::fs::File::create(&path).unwrap();
        let mut archive = ZipWriter::new(file);
        archive
            .start_file("word/document.xml", SimpleFileOptions::default())
            .unwrap();
        archive.write_all(br#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p><w:pPr><w:pStyle w:val="Heading1"/></w:pPr><w:r><w:t>2024</w:t></w:r></w:p><w:p><w:pPr><w:pStyle w:val="Heading3"/></w:pPr><w:r><w:t>March</w:t></w:r></w:p><w:p><w:pPr><w:pStyle w:val="Heading4"/></w:pPr><w:r><w:t>Synthetic entry</w:t></w:r></w:p><w:p><w:r><w:t>Synthetic memory</w:t></w:r></w:p></w:body></w:document>"#).unwrap();
        archive.finish().unwrap();
        assert!(inspect(path)
            .unwrap()
            .detections
            .iter()
            .any(|item| item.adapter_id == "diary-docx"));
        let path = directory.path().join("report.docx");
        let mut archive = ZipWriter::new(std::fs::File::create(&path).unwrap());
        archive
            .start_file("word/document.xml", SimpleFileOptions::default())
            .unwrap();
        archive.write_all(br#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p><w:r><w:t>Ordinary report</w:t></w:r></w:p></w:body></w:document>"#).unwrap();
        archive.finish().unwrap();
        assert!(!inspect(path)
            .unwrap()
            .detections
            .iter()
            .any(|item| item.adapter_id == "diary-docx"));
    }

    #[test]
    fn catalog_is_valid_and_open_contract_is_explicit() {
        let catalog: AdapterCatalog = serde_json::from_str(CATALOG).unwrap();
        assert_eq!(catalog.catalog_version, 1);
        assert!(catalog.output_contract.contains("Markdown"));
        assert!(catalog
            .adapters
            .iter()
            .any(|adapter| adapter.id == "banking-xlsx"));
        for executable in amara_unfold_adapters::EXECUTABLE_ADAPTERS {
            let definition = catalog
                .adapters
                .iter()
                .find(|adapter| adapter.id == *executable)
                .unwrap_or_else(|| panic!("{executable} is missing from the public catalog"));
            assert_eq!(definition.readiness, "ready");
            assert!(definition.command_template.is_some());
        }
    }

    #[test]
    fn banking_csv_is_recognized_by_columns_not_a_bank_specific_filename() {
        let directory = tempfile::tempdir().unwrap();
        let statement = directory.path().join("portable-export.csv");
        std::fs::write(
            &statement,
            "Report title\nTransaction Date;Debit Amount;Credit Amount;Currency;Payee\n2026-01-03;12.40;;EUR;Corner Cafe\n",
        )
        .unwrap();

        let inspection = inspect(statement).unwrap();
        let detection = inspection
            .detections
            .iter()
            .find(|item| item.adapter_id == "banking-csv")
            .unwrap();

        assert_eq!(detection.format, "Delimited transaction export (.csv)");
        assert!(detection
            .evidence
            .iter()
            .any(|evidence| evidence.contains("recognized transaction columns")));
    }

    #[test]
    fn unrelated_csv_is_not_mislabeled_as_banking() {
        let directory = tempfile::tempdir().unwrap();
        let export = directory.path().join("bank-notes.csv");
        std::fs::write(&export, "title,category\nA note,personal\n").unwrap();

        let inspection = inspect(export).unwrap();

        assert!(!inspection
            .detections
            .iter()
            .any(|item| item.adapter_id == "banking-csv"));
    }

    #[test]
    fn existing_photo_markdown_is_not_routed_to_extraction() {
        let directory = tempfile::tempdir().unwrap();
        std::fs::write(
            directory.path().join("manifest.yaml"),
            "schema_version: 1\n",
        )
        .unwrap();
        std::fs::write(directory.path().join("README.md"), "# Photos\n").unwrap();
        std::fs::create_dir(directory.path().join("library")).unwrap();

        let inspection = inspect(directory.path().to_path_buf()).unwrap();
        let detection = inspection
            .detections
            .iter()
            .find(|item| item.adapter_id == "foreverlane-photo-markdown-v1")
            .unwrap();

        assert_eq!(detection.stage, "open_markdown");
        assert_eq!(detection.readiness, "already_open");
        assert!(detection.command_preview.is_none());
    }

    #[test]
    fn one_google_takeout_can_expose_four_independent_offline_routes() {
        let directory = tempfile::tempdir().unwrap();
        let file = File::create(directory.path().join("takeout.zip")).unwrap();
        let mut archive = ZipWriter::new(file);
        for path in [
            "Takeout/Google Photos/2024/photo.jpg.supplemental-metadata.json",
            "Takeout/Gmail/All mail.mbox",
            "Takeout/YouTube and YouTube Music/subscriptions/subscriptions.csv",
            "Takeout/Drive/places.kmz",
        ] {
            archive
                .start_file(path, SimpleFileOptions::default())
                .unwrap();
            archive.write_all(b"fixture").unwrap();
        }
        archive.finish().unwrap();

        let inspection = inspect(directory.path().to_path_buf()).unwrap();
        let ids = inspection
            .detections
            .iter()
            .map(|item| item.adapter_id.as_str())
            .collect::<Vec<_>>();
        for expected in [
            "google-photos-takeout",
            "gmail-takeout",
            "youtube-takeout",
            "google-maps",
        ] {
            assert!(ids.contains(&expected), "missing {expected}: {ids:?}");
        }
    }

    #[test]
    fn standalone_kmz_and_kmz_folder_are_recognized_as_google_maps() {
        let directory = tempfile::tempdir().unwrap();
        let standalone = directory.path().join("My trip.kmz");
        std::fs::write(&standalone, b"fixture").unwrap();

        for source in [standalone, directory.path().to_path_buf()] {
            let inspection = inspect(source).unwrap();
            let detection = inspection
                .detections
                .iter()
                .find(|item| item.adapter_id == "google-maps")
                .unwrap();
            assert_eq!(detection.provider, "Google Maps");
            assert_eq!(detection.module_id, "google-maps");
            assert!(detection.format.contains("KMZ map file"));
        }
    }

    #[test]
    fn maps_csv_inside_zip_is_recognized_without_manual_unpacking() {
        let directory = tempfile::tempdir().unwrap();
        let file = File::create(directory.path().join("maps-export.zip")).unwrap();
        let mut archive = ZipWriter::new(file);
        archive
            .start_file(
                "Google Maps/ALL_Locations.csv",
                SimpleFileOptions::default(),
            )
            .unwrap();
        archive.write_all(b"Titel,Notiz,URL,Kommentar\n").unwrap();
        archive.finish().unwrap();

        let inspection = inspect(directory.path().join("maps-export.zip")).unwrap();
        let detection = inspection
            .detections
            .iter()
            .find(|item| item.adapter_id == "google-maps")
            .unwrap();
        assert_eq!(detection.format, "saved-place CSV inside ZIP");
    }

    #[test]
    fn localized_takeout_geojson_is_recognized_as_google_maps() {
        let directory = tempfile::tempdir().unwrap();
        let folder = directory.path().join("Takeout/Maps (Meine Orte)");
        std::fs::create_dir_all(&folder).unwrap();
        std::fs::write(
            folder.join("Gespeicherte Orte.json"),
            br#"{"type":"FeatureCollection","features":[]}"#,
        )
        .unwrap();

        let inspection = inspect(directory.path().to_path_buf()).unwrap();
        let detection = inspection
            .detections
            .iter()
            .find(|item| item.adapter_id == "google-maps")
            .unwrap();
        assert_eq!(detection.format, "Google Maps saved-places GeoJSON");
    }

    #[test]
    fn apple_mail_exported_mbox_package_is_recognized() {
        let directory = tempfile::tempdir().unwrap();
        let mailbox = directory.path().join("Inbox.mbox");
        std::fs::create_dir(&mailbox).unwrap();
        std::fs::write(mailbox.join("mbox"), b"From sender@example.test\n").unwrap();

        let inspection = inspect(mailbox).unwrap();
        let detection = inspection
            .detections
            .iter()
            .find(|item| item.adapter_id == "apple-mail-local")
            .unwrap();

        assert_eq!(detection.label, "Apple Mail exported mailbox");
        assert_eq!(detection.format, "Exported Apple Mail .mbox mailbox");
    }

    #[test]
    fn apple_mail_export_parent_folder_is_recognized() {
        let directory = tempfile::tempdir().unwrap();
        let export_folder = directory.path().join("Mail Export");
        let mailbox = export_folder.join("Inbox.mbox");
        std::fs::create_dir_all(&mailbox).unwrap();
        std::fs::write(mailbox.join("mbox"), b"From sender@example.test\n").unwrap();

        let inspection = inspect(export_folder).unwrap();
        let detection = inspection
            .detections
            .iter()
            .find(|item| item.adapter_id == "apple-mail-local")
            .unwrap();

        assert_eq!(detection.label, "Apple Mail exported mailbox");
        assert!(
            detection
                .evidence
                .iter()
                .any(|item| item == "contains Apple Mail .mbox package"),
            "{:?}",
            detection.evidence
        );
    }

    #[test]
    fn structured_airbnb_export_ranks_before_generic_embedded_photos() {
        let directory = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(directory.path().join("json")).unwrap();
        std::fs::create_dir_all(directory.path().join("images/profile")).unwrap();
        std::fs::write(directory.path().join("json/reservations.json"), "[]").unwrap();
        std::fs::write(directory.path().join("json/messages.json"), "[]").unwrap();
        std::fs::write(directory.path().join("json/reviews.json"), "[]").unwrap();
        std::fs::write(
            directory.path().join("images/profile/original.jpg"),
            b"image",
        )
        .unwrap();

        let inspection = inspect(directory.path().to_path_buf()).unwrap();
        let ids = inspection
            .detections
            .iter()
            .map(|item| item.adapter_id.as_str())
            .collect::<Vec<_>>();

        assert_eq!(ids.first(), Some(&"airbnb-json-takeout"));
        assert!(ids.contains(&"photo-media-folder"));
    }
}
