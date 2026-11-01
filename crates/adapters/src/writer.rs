// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::{model::slug, quality, AdapterError, ExtractionProgress, OpenEvent, QualitySummary};
use chrono::Local;
use serde::Serialize;
use std::{
    cell::RefCell,
    collections::HashSet,
    fs,
    path::{Component, Path, PathBuf},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WriteOutcome {
    Written,
    Unchanged,
    Duplicate,
}

pub struct PackageWriter {
    root: PathBuf,
    final_root: PathBuf,
    staging: Option<tempfile::TempDir>,
    adapter_id: String,
    source: PathBuf,
    written_this_run: RefCell<HashSet<PathBuf>>,
    written_ids_this_run: RefCell<HashSet<String>>,
    written_semantic_keys_this_run: RefCell<HashSet<String>>,
    known_event_ids: RefCell<HashSet<String>>,
    known_semantic_keys: RefCell<HashSet<String>>,
}

impl PackageWriter {
    pub fn new(destination: &Path, source: &Path, adapter_id: &str) -> Result<Self, AdapterError> {
        let source_name = source
            .file_stem()
            .and_then(|name| name.to_str())
            .unwrap_or("dataset");
        let package_name = format!(
            "{}-{}-unfold",
            safe_name(source_name),
            safe_name(adapter_id)
        );
        let export_date = Local::now().format("%Y-%m-%d").to_string();
        let final_root = destination
            .join(service_name(adapter_id))
            .join(export_date)
            .join(package_name);
        let parent = final_root.parent().expect("package parent");
        fs::create_dir_all(parent).map_err(|error| AdapterError::Output(error.to_string()))?;
        let staging = tempfile::Builder::new()
            .prefix(".unfold-staging-")
            .tempdir_in(parent)
            .map_err(|error| AdapterError::Output(error.to_string()))?;
        let root = staging.path().join("package");
        if final_root.exists() {
            copy_package(&final_root, &root)?;
        }
        fs::create_dir_all(root.join("events"))
            .map_err(|error| AdapterError::Output(error.to_string()))?;
        let (known_event_ids, known_semantic_keys) =
            read_event_identity_index(&root.join("events"))?;
        Ok(Self {
            root,
            final_root,
            staging: Some(staging),
            adapter_id: adapter_id.into(),
            source: source.into(),
            written_this_run: RefCell::new(HashSet::new()),
            written_ids_this_run: RefCell::new(HashSet::new()),
            written_semantic_keys_this_run: RefCell::new(HashSet::new()),
            known_event_ids: RefCell::new(known_event_ids),
            known_semantic_keys: RefCell::new(known_semantic_keys),
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn final_root(&self) -> &Path {
        &self.final_root
    }

    /// Publish only after parsing and validation finish. Errors/cancellation drop
    /// the private staging directory, preserving the last completed package.
    pub fn commit(&mut self) -> Result<(), AdapterError> {
        let previous = self
            .staging
            .as_ref()
            .expect("active staging")
            .path()
            .join("previous");
        let existed = self.final_root.exists();
        if existed {
            fs::rename(&self.final_root, &previous)
                .map_err(|error| AdapterError::Output(error.to_string()))?;
        }
        if let Err(error) = fs::rename(&self.root, &self.final_root) {
            if existed {
                if let Err(restore_error) = fs::rename(&previous, &self.final_root) {
                    let retained = self.staging.take().expect("active staging").keep();
                    return Err(AdapterError::Output(format!(
                        "package publish failed: {error}; restore failed: {restore_error}; previous package retained at {}",
                        retained.join("previous").display()
                    )));
                }
            }
            return Err(AdapterError::Output(error.to_string()));
        }
        Ok(())
    }

    /// Write a human-facing Markdown companion inside the portable package.
    ///
    /// Event Markdown remains canonical for Timeline indexing, while adapters
    /// may also emit compact catalogs and coverage notes that would be noisy as
    /// thousands of individual events. Relative paths are enforced so a source
    /// record can never escape the selected destination.
    pub fn write_markdown(&self, relative: &Path, content: &str) -> Result<(), AdapterError> {
        if relative.is_absolute()
            || relative.components().any(|component| {
                matches!(
                    component,
                    Component::ParentDir | Component::RootDir | Component::Prefix(_)
                )
            })
            || relative.extension().and_then(|value| value.to_str()) != Some("md")
        {
            return Err(AdapterError::Output(format!(
                "invalid portable Markdown path: {}",
                relative.display()
            )));
        }
        let path = self.root.join(relative);
        if path.is_file()
            && fs::read_to_string(&path)
                .map(|existing| existing == content)
                .unwrap_or(false)
        {
            return Ok(());
        }
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|error| AdapterError::Output(error.to_string()))?;
        }
        let temporary = path.with_extension("md.tmp");
        fs::write(&temporary, content).map_err(|error| AdapterError::Output(error.to_string()))?;
        fs::rename(&temporary, &path).map_err(|error| AdapterError::Output(error.to_string()))?;
        Ok(())
    }

    pub fn write_event(&self, event: &OpenEvent) -> Result<WriteOutcome, AdapterError> {
        let semantic_keys = google_maps_semantic_keys(event);
        let (year, month) = event.year_month();
        let directory = self.root.join("events").join(year).join(month);
        fs::create_dir_all(&directory).map_err(|error| AdapterError::Output(error.to_string()))?;
        let path = directory.join(event.filename());
        if path.exists() {
            self.remember_semantic_keys(&semantic_keys, false);
            return Ok(if self.written_this_run.borrow().contains(&path) {
                WriteOutcome::Duplicate
            } else {
                WriteOutcome::Unchanged
            });
        }
        if self.known_event_ids.borrow().contains(&event.id) {
            self.remember_semantic_keys(&semantic_keys, false);
            return Ok(if self.written_ids_this_run.borrow().contains(&event.id) {
                WriteOutcome::Duplicate
            } else {
                WriteOutcome::Unchanged
            });
        }
        if !semantic_keys.is_empty()
            && semantic_keys
                .iter()
                .any(|key| self.known_semantic_keys.borrow().contains(key))
        {
            let duplicate_this_run = semantic_keys
                .iter()
                .any(|key| self.written_semantic_keys_this_run.borrow().contains(key));
            // Remember every alias on the skipped bridge record. A GeoJSON
            // place can carry both coordinates and a Maps URL, allowing a KMZ
            // record and CSV record to converge without title-only guessing.
            self.remember_semantic_keys(&semantic_keys, duplicate_this_run);
            return Ok(if duplicate_this_run {
                WriteOutcome::Duplicate
            } else {
                WriteOutcome::Unchanged
            });
        }
        let yaml = event_yaml(event)?;
        let content = format!(
            "---\n{yaml}---\n\n# {}\n\n{}\n",
            event.title.trim(),
            event.body.trim()
        );
        let temporary = path.with_extension("md.tmp");
        fs::write(&temporary, content).map_err(|error| AdapterError::Output(error.to_string()))?;
        fs::rename(temporary, &path).map_err(|error| AdapterError::Output(error.to_string()))?;
        self.written_this_run.borrow_mut().insert(path);
        self.written_ids_this_run
            .borrow_mut()
            .insert(event.id.clone());
        self.known_event_ids.borrow_mut().insert(event.id.clone());
        self.remember_semantic_keys(&semantic_keys, true);
        Ok(WriteOutcome::Written)
    }

    fn remember_semantic_keys(&self, keys: &HashSet<String>, written_this_run: bool) {
        self.known_semantic_keys
            .borrow_mut()
            .extend(keys.iter().cloned());
        if written_this_run {
            self.written_semantic_keys_this_run
                .borrow_mut()
                .extend(keys.iter().cloned());
        }
    }

    pub fn finish(&self, progress: &ExtractionProgress) -> Result<QualitySummary, AdapterError> {
        remove_platform_metadata(&self.root)?;
        let quality = self.validate_stable_package(progress)?;
        quality::write_reports(&self.root, &quality)?;
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Manifest<'a> {
            schema_version: u8,
            adapter_id: &'a str,
            source_path: String,
            output_contract: &'a str,
            records_seen: u64,
            source_records_included: u64,
            events_written: u64,
            events_skipped: u64,
            errors: u64,
            skip_reasons: &'a std::collections::BTreeMap<String, u64>,
            quality: &'a QualitySummary,
            network_used: bool,
        }
        let manifest = Manifest {
            schema_version: 1,
            adapter_id: &self.adapter_id,
            source_path: self.source.to_string_lossy().into_owned(),
            output_contract: "Amara Unfold Open Event Schema v1 Markdown with YAML frontmatter",
            records_seen: progress.records_seen,
            source_records_included: progress.source_records_included,
            events_written: progress.events_written,
            events_skipped: progress.events_skipped,
            errors: progress.errors,
            skip_reasons: &progress.skip_reasons,
            quality: &quality,
            network_used: false,
        };
        let yaml = serde_yaml::to_string(&manifest)
            .map_err(|error| AdapterError::Output(error.to_string()))?;
        fs::write(self.root.join("manifest.yaml"), yaml)
            .map_err(|error| AdapterError::Output(error.to_string()))?;
        let readme = format!(
            "# Amara Unfold open Markdown\n\nThis folder is the durable result of the `{}` offline adapter.\n\n- One Timeline event per Markdown file under `events/YYYY/MM/`.\n- Compact module catalogs and coverage notes may sit beside `events/` when thousands of source records would be noisy as individual Timeline cards.\n- Event filenames combine sortable date/time, source, a readable title, and a stable short ID.\n- YAML frontmatter follows the open Amara Unfold Event Schema v1.\n- `provenance.raw_path` and `provenance.raw_hash` preserve traceability.\n- No network service or Amara product is required to read or use these files.\n- Re-running the adapter is idempotent; existing event IDs are preserved even when an older export used the legacy filename format.\n- `manifest.yaml` accounts for every written/skipped record and groups skips by reason.\n- `quality-report.md` and `quality-report.json` are generated automatically after extraction.\n\nFor KMZ and other overlapping exports, `duplicate_event` means the same deterministic event appeared more than once; `unchanged_rerun` means a prior output file was preserved. These are not silent data loss.\n",
            self.adapter_id
        );
        fs::write(self.root.join("README.md"), readme)
            .map_err(|error| AdapterError::Output(error.to_string()))?;
        remove_platform_metadata(&self.root)?;
        Ok(quality)
    }

    fn validate_stable_package(
        &self,
        progress: &ExtractionProgress,
    ) -> Result<QualitySummary, AdapterError> {
        let mut previous =
            quality::validate_package(&self.root, &self.adapter_id, &self.source, progress)?;
        for _ in 0..2 {
            let current =
                quality::validate_package(&self.root, &self.adapter_id, &self.source, progress)?;
            if current == previous {
                return Ok(current);
            }
            previous = current;
        }
        Err(AdapterError::Output(
            "emitted files changed while quality validation was running; retry the export in an isolated destination"
                .into(),
        ))
    }
}

fn copy_package(source: &Path, destination: &Path) -> Result<(), AdapterError> {
    for entry in walkdir::WalkDir::new(source).follow_links(false) {
        let entry = entry.map_err(|error| AdapterError::Output(error.to_string()))?;
        let target = destination.join(entry.path().strip_prefix(source).expect("package path"));
        if entry.file_type().is_symlink() {
            return Err(AdapterError::Output(
                "existing output contains a symbolic link; choose an isolated destination".into(),
            ));
        }
        if entry.file_type().is_dir() {
            fs::create_dir_all(&target).map_err(|error| AdapterError::Output(error.to_string()))?;
        } else if entry.file_type().is_file() {
            fs::copy(entry.path(), &target)
                .map_err(|error| AdapterError::Output(error.to_string()))?;
        }
    }
    Ok(())
}

fn read_event_identity_index(
    events_root: &Path,
) -> Result<(HashSet<String>, HashSet<String>), AdapterError> {
    let mut ids = HashSet::new();
    let mut semantic_keys = HashSet::new();
    for entry in walkdir::WalkDir::new(events_root)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| {
            entry.file_type().is_file()
                && entry.path().extension().and_then(|value| value.to_str()) == Some("md")
        })
    {
        let content = fs::read_to_string(entry.path()).map_err(|error| {
            AdapterError::Output(format!(
                "could not read existing event {}: {error}",
                entry.path().display()
            ))
        })?;
        if let Some(id) = frontmatter_event_id(&content) {
            ids.insert(id);
        }
        if let Some(yaml) = frontmatter_yaml(&content) {
            if let Ok(event) = serde_yaml::from_str::<OpenEvent>(yaml) {
                semantic_keys.extend(google_maps_semantic_keys(&event));
            }
        }
    }
    Ok((ids, semantic_keys))
}

fn frontmatter_yaml(content: &str) -> Option<&str> {
    content
        .strip_prefix("---\n")
        .and_then(|content| content.split_once("\n---\n").map(|parts| parts.0))
}

fn google_maps_semantic_keys(event: &OpenEvent) -> HashSet<String> {
    let module = event.module.to_ascii_lowercase().replace('_', "-");
    if !["google-maps", "google-maps-pois", "kmz", "maps"].contains(&module.as_str()) {
        return HashSet::new();
    }
    let title = {
        let value = slug(&event.title);
        if value.is_empty() {
            "saved-place".to_owned()
        } else {
            value
        }
    };
    let mut keys = event
        .places
        .iter()
        .filter(|place| place.starts_with("pg-"))
        .map(|place| format!("google-maps:coordinate:{place}:title:{title}"))
        .collect::<HashSet<_>>();
    keys.extend(event.links.iter().filter_map(|link| {
        canonical_google_maps_url(link).map(|url| format!("google-maps:url:{url}"))
    }));
    keys
}

fn canonical_google_maps_url(value: &str) -> Option<String> {
    let lower = value
        .trim()
        .trim_matches(['\"', '\'', '<', '>', '(', ')', '.', ',', ';'])
        .to_ascii_lowercase();
    if !(lower.contains("google.com/maps") || lower.contains("maps.google.")) {
        return None;
    }
    if let Some(index) = lower.find("cid=") {
        let cid = lower[index + 4..]
            .split(['&', '#', '/', '?'])
            .next()
            .unwrap_or_default();
        if !cid.is_empty() {
            return Some(format!("cid:{cid}"));
        }
    }
    if let Some(index) = lower.find("!1s") {
        let place_id = lower[index + 3..]
            .split(['!', '&', '#', '/', '?'])
            .next()
            .unwrap_or_default();
        if let Some(hexadecimal_cid) = place_id
            .rsplit(':')
            .next()
            .and_then(|value| value.strip_prefix("0x"))
        {
            if let Ok(cid) = u128::from_str_radix(hexadecimal_cid, 16) {
                return Some(format!("cid:{cid}"));
            }
        }
        if !place_id.is_empty() {
            return Some(format!("place-id:{place_id}"));
        }
    }
    Some(
        lower
            .strip_prefix("https://")
            .or_else(|| lower.strip_prefix("http://"))
            .unwrap_or(&lower)
            .trim_start_matches("www.")
            .trim_end_matches('/')
            .to_owned(),
    )
}

fn frontmatter_event_id(content: &str) -> Option<String> {
    let mut lines = content.lines();
    if lines.next() != Some("---") {
        return None;
    }
    for line in lines {
        if line == "---" {
            break;
        }
        if let Some(id) = line.strip_prefix("id: ") {
            return Some(id.trim().trim_matches(['\'', '"']).to_owned());
        }
    }
    None
}

pub(crate) fn event_yaml(event: &OpenEvent) -> Result<String, AdapterError> {
    let yaml =
        serde_yaml::to_string(event).map_err(|error| AdapterError::Output(error.to_string()))?;
    let mut output = Vec::new();
    for line in yaml.lines() {
        if line.starts_with("date: ") {
            output.push(format!("date: {}", yaml_string(&event.date)?));
        } else if line.starts_with("time: ") {
            output.push(format!(
                "time: {}",
                yaml_string(event.time.as_deref().unwrap_or_default())?
            ));
        } else if line.starts_with("end_date: ") {
            output.push(format!(
                "end_date: {}",
                yaml_string(event.end_date.as_deref().unwrap_or_default())?
            ));
        } else if line.starts_with("  ingested_at: ") {
            output.push(format!(
                "  ingested_at: {}",
                yaml_string(&event.provenance.ingested_at)?
            ));
        } else {
            output.push(line.to_owned());
        }
    }
    Ok(format!("{}\n", output.join("\n")))
}

fn yaml_string(value: &str) -> Result<String, AdapterError> {
    serde_json::to_string(value).map_err(|error| AdapterError::Output(error.to_string()))
}

fn service_name(adapter_id: &str) -> &str {
    match adapter_id {
        "photo-media-folder" | "google-photos-takeout" => "photos",
        "gmail-takeout" | "pst-mail" | "apple-mail-local" => "mail",
        "apple-calendar-local" => "calendar",
        "apple-contacts-local" => "contacts",
        "youtube-takeout" => "youtube",
        "google-maps" | "google-drive-kmz" | "google-maps-saved-csv" | "google-maps-pois-json" => {
            "google-maps"
        }
        "apple-music-tsv" | "apple-music-local-library" => "apple-music",
        "immich-local-library" => "immich",
        "banking-xlsx" | "banking-csv" => "banking",
        "airbnb-json-takeout" => "airbnb",
        "diary-docx" => "diary",
        _ => "other",
    }
}

fn remove_platform_metadata(root: &Path) -> Result<(), AdapterError> {
    for entry in walkdir::WalkDir::new(root)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
    {
        let name = entry.file_name().to_string_lossy();
        if name.starts_with("._") || name == ".DS_Store" {
            fs::remove_file(entry.path()).map_err(|error| {
                AdapterError::Output(format!(
                    "could not remove filesystem metadata {}: {error}",
                    entry.path().display()
                ))
            })?;
        }
    }
    Ok(())
}

fn safe_name(value: &str) -> String {
    let output = value
        .to_lowercase()
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character
            } else {
                '-'
            }
        })
        .collect::<String>();
    output
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::EventInput;

    #[test]
    fn output_cleanup_removes_appledouble_without_touching_markdown() {
        let directory = tempfile::tempdir().unwrap();
        fs::write(directory.path().join("event.md"), "# Event\n").unwrap();
        fs::write(directory.path().join("._event.md"), b"metadata").unwrap();

        remove_platform_metadata(directory.path()).unwrap();

        assert!(directory.path().join("event.md").is_file());
        assert!(!directory.path().join("._event.md").exists());
    }

    #[test]
    fn event_yaml_quotes_partial_and_unknown_dates_for_portable_parsers() {
        let event = OpenEvent::new(EventInput {
            date: "1974-09",
            time: None,
            end_date: None,
            source: "diary",
            module: "diary",
            event_type: "diary_entry",
            title: "September memory",
            raw_path: "/source/diary.docx",
            raw_bytes: b"memory",
            body: "Verbatim memory".into(),
            actors: Vec::new(),
            places: Vec::new(),
            tags: vec!["diary".into()],
            confidence: "high",
        });
        let yaml = event_yaml(&event).unwrap();
        assert!(yaml.contains("date: \"1974-09-00\""));
        assert!(yaml.contains("ingested_at: \""));
    }

    #[test]
    fn rerun_recognizes_event_id_from_legacy_filename_without_creating_duplicate() {
        let destination = tempfile::tempdir().unwrap();
        let source = destination.path().join("diary.docx");
        fs::write(&source, b"source").unwrap();
        let event = OpenEvent::new(EventInput {
            date: "0000-00-00",
            time: None,
            end_date: None,
            source: "diary",
            module: "diary",
            event_type: "diary_entry",
            title: "A readable title",
            raw_path: source.to_str().unwrap(),
            raw_bytes: b"source record",
            body: "Verbatim memory".into(),
            actors: Vec::new(),
            places: Vec::new(),
            tags: vec!["diary".into()],
            confidence: "high",
        });
        let mut first_writer =
            PackageWriter::new(destination.path(), &source, "diary-docx").unwrap();
        let legacy_directory = first_writer.root().join("events/0000/00");
        fs::create_dir_all(&legacy_directory).unwrap();
        let hash = event.id.rsplit('-').next().unwrap();
        let legacy_path = legacy_directory.join(format!("00000000-0000-diary-{hash}.md"));
        fs::write(
            &legacy_path,
            format!("---\nid: {}\n---\n\n# A readable title\n", event.id),
        )
        .unwrap();

        first_writer.commit().unwrap();
        let rerun_writer = PackageWriter::new(destination.path(), &source, "diary-docx").unwrap();
        assert_eq!(
            rerun_writer.write_event(&event).unwrap(),
            WriteOutcome::Unchanged
        );
        assert!(rerun_writer
            .root()
            .join("events/0000/00")
            .join(legacy_path.file_name().unwrap())
            .is_file());
        assert!(!legacy_directory.join(event.filename()).exists());
    }
}
