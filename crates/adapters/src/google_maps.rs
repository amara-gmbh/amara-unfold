// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::{kmz, maps_csv, pois, AdapterError, ExtractionProgress, ProgressCallback};
use std::{path::Path, sync::atomic::AtomicBool};

/// Normalize every supported Google Maps packaging route into one package and
/// one provider. Input format remains visible in recognition and provenance;
/// it must not split the user's Maps data into separate Timeline providers.
pub fn extract(
    source: &Path,
    package: &crate::writer::PackageWriter,
    cancel: &AtomicBool,
    progress: &mut ExtractionProgress,
    callback: &mut ProgressCallback<'_>,
) -> Result<(), AdapterError> {
    let starting_records = progress.records_seen;

    // JSON may carry both a stable Maps URL and coordinates, so processing it
    // first lets it bridge equivalent CSV and KMZ records without relying on
    // titles alone (which could incorrectly merge distinct places).
    pois::extract(source, package, cancel, progress, callback)?;
    kmz::extract(source, package, cancel, progress, callback)?;
    maps_csv::extract(source, package, cancel, progress, callback)?;

    if progress.records_seen == starting_records {
        return Err(AdapterError::Parse(
            "No supported Google Maps records were found in this source".into(),
        ));
    }
    Ok(())
}
