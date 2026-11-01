// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::{AdapterError, ExtractionProgress, ProgressCallback};
use std::{path::Path, sync::atomic::AtomicBool};

pub fn extract(
    source: &Path,
    package: &crate::writer::PackageWriter,
    cancel: &AtomicBool,
    progress: &mut ExtractionProgress,
    callback: &mut ProgressCallback<'_>,
) -> Result<(), AdapterError> {
    let mbox_sources = crate::mail::extract_mbox_sources(
        source,
        "apple-mail",
        package,
        cancel,
        progress,
        callback,
    )?;
    if mbox_sources > 0 {
        return Ok(());
    }

    Err(AdapterError::Input(format!(
        "No readable Apple Mail export was found in {}. In Apple Mail, select a mailbox, choose Mailbox › Export Mailbox…, then choose the exported .mbox folder in Unfold.",
        source.display()
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_export_error_is_actionable() {
        let source = Path::new("/Users/example/Library/Mail");
        let package_root = tempfile::TempDir::new().unwrap();
        let package =
            crate::writer::PackageWriter::new(package_root.path(), source, "apple-mail-local")
                .unwrap();
        let mut progress = ExtractionProgress {
            adapter_id: "apple-mail-local".into(),
            phase: "extracting".into(),
            current_item: None,
            records_seen: 0,
            source_records_included: 0,
            events_written: 0,
            events_skipped: 0,
            errors: 0,
            skip_reasons: Default::default(),
            total_hint: None,
            message: "Preparing local extraction".into(),
        };
        let mut callback = |_progress: ExtractionProgress| {};
        let error = extract(
            source,
            &package,
            &AtomicBool::new(false),
            &mut progress,
            &mut callback,
        )
        .expect_err("raw Mail store should not be treated as a supported source");
        let message = error.to_string();

        assert!(message.contains("No readable Apple Mail export"));
        assert!(!message.contains("Full Disk Access"));
        assert!(message.contains("Mailbox › Export Mailbox"));
    }
}
