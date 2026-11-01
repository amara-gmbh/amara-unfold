// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::AdapterError;
use std::{
    fs::{self, File},
    io::Read,
    path::{Path, PathBuf},
};
use walkdir::WalkDir;
use zip::ZipArchive;

pub const MAX_ARCHIVE_ENTRIES: usize = 100_000;
pub const MAX_BLOB_BYTES: u64 = 512 * 1024 * 1024;
pub const MAX_ARCHIVE_TOTAL_BYTES: u64 = 2 * 1024 * 1024 * 1024;
pub const MAX_COMPRESSION_RATIO: u64 = 10_000;

#[derive(Debug)]
pub struct InputBlob {
    pub raw_path: String,
    pub bytes: Vec<u8>,
}

pub fn files_with_extensions(source: &Path, extensions: &[&str]) -> Vec<PathBuf> {
    if source.is_file() {
        return extension_matches(source, extensions)
            .then(|| source.to_path_buf())
            .into_iter()
            .collect();
    }
    WalkDir::new(source)
        .follow_links(false)
        .sort_by_file_name()
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .map(|entry| entry.into_path())
        .filter(|path| !is_sidecar(path) && extension_matches(path, extensions))
        .collect()
}

pub fn matching_blobs<F>(source: &Path, mut predicate: F) -> Result<Vec<InputBlob>, AdapterError>
where
    F: FnMut(&str) -> bool,
{
    let mut blobs = Vec::new();
    for path in files_with_extensions(source, &["json", "geojson", "csv", "tsv", "mbox", "kmz"]) {
        let display = path.to_string_lossy().into_owned();
        if predicate(&display) {
            validate_file_size(&path, MAX_BLOB_BYTES, "text input")?;
            blobs.push(InputBlob {
                raw_path: display,
                bytes: fs::read(&path).map_err(|error| AdapterError::Input(error.to_string()))?,
            });
        }
    }
    for archive_path in files_with_extensions(source, &["zip"]) {
        let file =
            File::open(&archive_path).map_err(|error| AdapterError::Input(error.to_string()))?;
        let mut archive =
            ZipArchive::new(file).map_err(|error| AdapterError::Input(error.to_string()))?;
        if archive.len() > MAX_ARCHIVE_ENTRIES {
            return Err(AdapterError::Input(format!(
                "{} contains too many ZIP entries ({} > {})",
                archive_path.display(),
                archive.len(),
                MAX_ARCHIVE_ENTRIES
            )));
        }
        let mut selected_total = 0_u64;
        for index in 0..archive.len() {
            let entry = archive
                .by_index(index)
                .map_err(|error| AdapterError::Input(error.to_string()))?;
            let entry_name = entry
                .name()
                .map_err(|error| AdapterError::Input(error.to_string()))?
                .into_owned();
            if entry.is_dir()
                || !predicate(&entry_name)
                || entry_name
                    .rsplit('/')
                    .next()
                    .is_some_and(|name| name.starts_with("._"))
            {
                continue;
            }
            validate_entry_limits(
                &entry_name,
                entry.size(),
                entry.compressed_size(),
                MAX_BLOB_BYTES,
            )?;
            selected_total = selected_total.checked_add(entry.size()).ok_or_else(|| {
                AdapterError::Input("selected ZIP content size overflowed".into())
            })?;
            if selected_total > MAX_ARCHIVE_TOTAL_BYTES {
                return Err(AdapterError::Input(format!(
                    "{} expands beyond the {} byte selected-content limit",
                    archive_path.display(),
                    MAX_ARCHIVE_TOTAL_BYTES
                )));
            }
            let mut bytes = Vec::with_capacity(entry.size().min(8_000_000) as usize);
            entry
                .take(MAX_BLOB_BYTES + 1)
                .read_to_end(&mut bytes)
                .map_err(|error| AdapterError::Input(error.to_string()))?;
            if bytes.len() as u64 > MAX_BLOB_BYTES {
                return Err(AdapterError::Input(format!(
                    "ZIP entry {entry_name} exceeded the read limit"
                )));
            }
            blobs.push(InputBlob {
                raw_path: format!("{}!{entry_name}", archive_path.display()),
                bytes,
            });
        }
    }
    Ok(blobs)
}

pub fn validate_file_size(path: &Path, limit: u64, label: &str) -> Result<(), AdapterError> {
    let size = fs::metadata(path)
        .map_err(|error| AdapterError::Input(error.to_string()))?
        .len();
    if size > limit {
        return Err(AdapterError::Input(format!(
            "{label} {} is {size} bytes; limit is {limit}",
            path.display()
        )));
    }
    Ok(())
}

pub fn validate_zip_container(
    path: &Path,
    max_entries: usize,
    max_entry_bytes: u64,
    max_total_bytes: u64,
) -> Result<(), AdapterError> {
    let file = File::open(path).map_err(|error| AdapterError::Input(error.to_string()))?;
    let mut archive =
        ZipArchive::new(file).map_err(|error| AdapterError::Input(error.to_string()))?;
    if archive.len() > max_entries {
        return Err(AdapterError::Input(format!(
            "{} contains too many archive entries ({} > {max_entries})",
            path.display(),
            archive.len()
        )));
    }
    let mut total = 0_u64;
    for index in 0..archive.len() {
        let entry = archive
            .by_index(index)
            .map_err(|error| AdapterError::Input(error.to_string()))?;
        if entry.is_dir() {
            continue;
        }
        let entry_name = entry
            .name()
            .map_err(|error| AdapterError::Input(error.to_string()))?;
        validate_entry_limits(
            &entry_name,
            entry.size(),
            entry.compressed_size(),
            max_entry_bytes,
        )?;
        total = total
            .checked_add(entry.size())
            .ok_or_else(|| AdapterError::Input("archive size overflowed".into()))?;
        if total > max_total_bytes {
            return Err(AdapterError::Input(format!(
                "{} expands beyond the {max_total_bytes} byte limit",
                path.display()
            )));
        }
    }
    Ok(())
}

pub(crate) fn validate_entry_limits(
    name: &str,
    size: u64,
    compressed_size: u64,
    max_entry_bytes: u64,
) -> Result<(), AdapterError> {
    if Path::new(name).is_absolute()
        || Path::new(name)
            .components()
            .any(|component| matches!(component, std::path::Component::ParentDir))
    {
        return Err(AdapterError::Input(format!(
            "archive entry has an unsafe path: {name}"
        )));
    }
    if size > max_entry_bytes {
        return Err(AdapterError::Input(format!(
            "archive entry {name} expands to {size} bytes; limit is {max_entry_bytes}"
        )));
    }
    if compressed_size > 0
        && u128::from(size)
            > u128::from(compressed_size).saturating_mul(u128::from(MAX_COMPRESSION_RATIO))
    {
        return Err(AdapterError::Input(format!(
            "archive entry {name} exceeds the compression-ratio limit"
        )));
    }
    Ok(())
}

fn extension_matches(path: &Path, extensions: &[&str]) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            extensions
                .iter()
                .any(|candidate| extension.eq_ignore_ascii_case(candidate))
        })
}

fn is_sidecar(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.starts_with("._") || name == ".DS_Store")
}

#[cfg(test)]
mod archive_security_tests {
    use super::{validate_entry_limits, validate_zip_container};
    use std::io::Write;
    use zip::{write::SimpleFileOptions, ZipWriter};

    #[test]
    fn archive_limits_reject_parent_paths_and_expansion() {
        assert!(validate_entry_limits("../outside.txt", 1, 1, 100).is_err());
        assert!(validate_entry_limits("valid.txt", 101, 1, 100).is_err());
        assert!(validate_entry_limits("valid.txt", 10_001, 1, 20_000).is_err());
        assert!(validate_entry_limits("folder/valid.txt", 10, 10, 100).is_ok());
    }

    #[test]
    fn normal_zip_remains_accepted_after_dependency_upgrade() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let path = directory.path().join("input.zip");
        let file = std::fs::File::create(&path).expect("temporary ZIP");
        let mut writer = ZipWriter::new(file);
        writer
            .start_file("folder/input.txt", SimpleFileOptions::default())
            .expect("ZIP entry");
        writer.write_all(b"example").expect("ZIP contents");
        writer.finish().expect("finish ZIP");
        assert!(validate_zip_container(&path, 5, 100, 100).is_ok());
    }
}
