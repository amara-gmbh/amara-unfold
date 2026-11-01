// SPDX-License-Identifier: AGPL-3.0-or-later

use amara_unfold_core::{DiscoveredAsset, MediaKind};
use amara_unfold_extractors::{media_kind_for_extension, ExtractorRegistry};
use std::{
    fs::File,
    io::Read,
    path::Path,
    sync::atomic::{AtomicBool, Ordering},
    time::SystemTime,
};
use walkdir::WalkDir;

#[derive(Debug, Default)]
pub struct ScanOutput {
    pub assets: Vec<DiscoveredAsset>,
    pub errors: Vec<String>,
    pub cancelled: bool,
}

/// Scanning has no storage dependency. This makes traversal deterministic and
/// testable, and prevents partial database state when cancellation occurs.
pub trait ArchiveScanner: Send + Sync {
    fn scan(
        &self,
        root: &Path,
        cancel: &AtomicBool,
        progress: &mut dyn FnMut(&Path, u64, u64, u64),
    ) -> Result<ScanOutput, String>;
}

pub struct LocalArchiveScanner {
    registry: ExtractorRegistry,
}

impl Default for LocalArchiveScanner {
    fn default() -> Self {
        Self {
            registry: ExtractorRegistry::standard(),
        }
    }
}

impl LocalArchiveScanner {
    pub fn new(registry: ExtractorRegistry) -> Self {
        Self { registry }
    }
}

impl ArchiveScanner for LocalArchiveScanner {
    fn scan(
        &self,
        root: &Path,
        cancel: &AtomicBool,
        progress: &mut dyn FnMut(&Path, u64, u64, u64),
    ) -> Result<ScanOutput, String> {
        if !root.is_dir() {
            return Err(format!("{} is not a readable folder", root.display()));
        }

        let mut output = ScanOutput::default();
        let mut files_seen = 0_u64;
        let mut media_found = 0_u64;

        // Symlinks are intentionally not followed. Personal archives often
        // contain aliases that point outside the selected privacy boundary.
        for entry in WalkDir::new(root).follow_links(false) {
            if cancel.load(Ordering::Relaxed) {
                output.cancelled = true;
                break;
            }
            let entry = match entry {
                Ok(value) => value,
                Err(error) => {
                    output.errors.push(error.to_string());
                    continue;
                }
            };
            if !entry.file_type().is_file() {
                continue;
            }

            files_seen += 1;
            let path = entry.path();
            let extension = path
                .extension()
                .and_then(|value| value.to_str())
                .unwrap_or_default()
                .to_ascii_lowercase();
            let media_kind = media_kind_for_extension(&extension);
            if media_kind != MediaKind::Unknown {
                media_found += 1;
            }
            progress(path, files_seen, media_found, output.errors.len() as u64);

            let filesystem_metadata = match entry.metadata() {
                Ok(value) => value,
                Err(error) => {
                    output.errors.push(format!("{}: {error}", path.display()));
                    continue;
                }
            };
            let extracted = self.registry.extract(path, media_kind);
            let content_hash = if media_kind != MediaKind::Unknown {
                match hash_file(path) {
                    Ok(value) => Some(value),
                    Err(error) => {
                        output.errors.push(format!("{}: {error}", path.display()));
                        None
                    }
                }
            } else {
                None
            };

            output.assets.push(DiscoveredAsset {
                path: path.to_path_buf(),
                filename: entry.file_name().to_string_lossy().into_owned(),
                extension,
                media_kind,
                size_bytes: filesystem_metadata.len(),
                created_at: filesystem_metadata.created().ok().map(system_time_string),
                modified_at: filesystem_metadata.modified().ok().map(system_time_string),
                content_hash,
                metadata: extracted,
            });
        }
        if files_seen == 0 && !output.errors.is_empty() && !output.cancelled {
            return Err(output.errors.join("; "));
        }
        Ok(output)
    }
}

fn hash_file(path: &Path) -> Result<String, String> {
    let mut file = File::open(path).map_err(|error| error.to_string())?;
    let mut hasher = blake3::Hasher::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let count = file.read(&mut buffer).map_err(|error| error.to_string())?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }
    Ok(hasher.finalize().to_hex().to_string())
}

fn system_time_string(value: SystemTime) -> String {
    let date: chrono::DateTime<chrono::Utc> = value.into();
    date.to_rfc3339()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scanner_recurses_and_indexes_unknown_files() {
        let directory = tempfile::tempdir().unwrap();
        std::fs::create_dir(directory.path().join("nested")).unwrap();
        std::fs::write(
            directory.path().join("nested/photo.jpg"),
            b"not really an image",
        )
        .unwrap();
        std::fs::write(directory.path().join("notes.txt"), b"private notes").unwrap();
        let scanner = LocalArchiveScanner::default();
        let output = scanner
            .scan(
                directory.path(),
                &AtomicBool::new(false),
                &mut |_, _, _, _| {},
            )
            .unwrap();
        assert_eq!(output.assets.len(), 2);
        assert!(output
            .assets
            .iter()
            .any(|asset| asset.media_kind == MediaKind::Image));
        assert!(output
            .assets
            .iter()
            .any(|asset| asset.media_kind == MediaKind::Unknown));
    }

    #[test]
    fn scanner_honors_cancellation_before_traversal() {
        let directory = tempfile::tempdir().unwrap();
        std::fs::write(directory.path().join("photo.jpg"), b"data").unwrap();
        let scanner = LocalArchiveScanner::default();
        let output = scanner
            .scan(
                directory.path(),
                &AtomicBool::new(true),
                &mut |_, _, _, _| {},
            )
            .unwrap();
        assert!(output.cancelled);
        assert!(output.assets.is_empty());
    }

    #[test]
    fn scanner_honors_cancellation_during_traversal() {
        let directory = tempfile::tempdir().unwrap();
        for index in 0..10 {
            std::fs::write(directory.path().join(format!("{index:02}.jpg")), b"fixture").unwrap();
        }
        let scanner = LocalArchiveScanner::default();
        let cancelled = AtomicBool::new(false);
        let output = scanner
            .scan(directory.path(), &cancelled, &mut |_, files_seen, _, _| {
                if files_seen == 1 {
                    cancelled.store(true, Ordering::Relaxed);
                }
            })
            .unwrap();
        assert!(output.cancelled);
        assert_eq!(output.assets.len(), 1);
    }

    #[cfg(unix)]
    #[test]
    fn scanner_does_not_cross_symbolic_link_boundary() {
        use std::os::unix::fs::symlink;

        let selected = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        std::fs::write(outside.path().join("private.jpg"), b"outside fixture").unwrap();
        symlink(outside.path(), selected.path().join("outside-link")).unwrap();
        std::fs::write(selected.path().join("inside.jpg"), b"inside fixture").unwrap();

        let output = LocalArchiveScanner::default()
            .scan(
                selected.path(),
                &AtomicBool::new(false),
                &mut |_, _, _, _| {},
            )
            .unwrap();
        assert_eq!(output.assets.len(), 1);
        assert_eq!(output.assets[0].filename, "inside.jpg");
    }
}
