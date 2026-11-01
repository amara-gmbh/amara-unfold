// SPDX-License-Identifier: AGPL-3.0-or-later

use amara_unfold_core::{ExtractedMetadata, MediaKind};
use exif::{In, Reader, Tag, Value};
use image::ImageReader;
use std::{fs::File, io::BufReader, path::Path};

/// Metadata extractors are deliberately small and capability-based. A failed
/// extractor never prevents an asset from being indexed: filesystem facts are
/// still useful evidence, and extraction can be retried by a future version.
pub trait MetadataExtractor: Send + Sync {
    fn name(&self) -> &'static str;
    fn supports(&self, path: &Path, kind: MediaKind) -> bool;
    fn extract(&self, path: &Path) -> Result<ExtractedMetadata, String>;
}

#[derive(Default)]
pub struct ExtractorRegistry {
    extractors: Vec<Box<dyn MetadataExtractor>>,
}

impl ExtractorRegistry {
    pub fn standard() -> Self {
        Self {
            extractors: vec![Box::new(ImageExtractor)],
        }
    }

    pub fn register(&mut self, extractor: Box<dyn MetadataExtractor>) {
        self.extractors.push(extractor);
    }

    pub fn extract(&self, path: &Path, kind: MediaKind) -> ExtractedMetadata {
        for extractor in &self.extractors {
            if extractor.supports(path, kind) {
                return extractor
                    .extract(path)
                    .unwrap_or_else(|error| ExtractedMetadata {
                        extractor: extractor.name().into(),
                        extra: serde_json::json!({ "error": error }),
                        ..ExtractedMetadata::default()
                    });
            }
        }
        ExtractedMetadata {
            extractor: "filesystem".into(),
            extra: serde_json::json!({}),
            ..ExtractedMetadata::default()
        }
    }
}

pub struct ImageExtractor;

impl MetadataExtractor for ImageExtractor {
    fn name(&self) -> &'static str {
        "image+exif"
    }

    fn supports(&self, _path: &Path, kind: MediaKind) -> bool {
        kind == MediaKind::Image
    }

    fn extract(&self, path: &Path) -> Result<ExtractedMetadata, String> {
        let dimensions = ImageReader::open(path)
            .ok()
            .and_then(|reader| reader.into_dimensions().ok());
        let mut result = ExtractedMetadata {
            width: dimensions.map(|item| item.0),
            height: dimensions.map(|item| item.1),
            extractor: self.name().into(),
            extra: serde_json::json!({}),
            ..ExtractedMetadata::default()
        };

        let Ok(file) = File::open(path) else {
            return Ok(result);
        };
        let Ok(exif) = Reader::new().read_from_container(&mut BufReader::new(file)) else {
            return Ok(result);
        };

        result.captured_at = display_field(&exif, Tag::DateTimeOriginal);
        result.device_make = display_field(&exif, Tag::Make);
        result.device_model = display_field(&exif, Tag::Model);
        result.latitude = gps_coordinate(&exif, Tag::GPSLatitude, Tag::GPSLatitudeRef);
        result.longitude = gps_coordinate(&exif, Tag::GPSLongitude, Tag::GPSLongitudeRef);
        Ok(result)
    }
}

fn display_field(exif: &exif::Exif, tag: Tag) -> Option<String> {
    exif.get_field(tag, In::PRIMARY)
        .map(|field| {
            field
                .display_value()
                .with_unit(exif)
                .to_string()
                .trim_matches('"')
                .trim()
                .to_owned()
        })
        .filter(|value| !value.is_empty())
}

fn gps_coordinate(exif: &exif::Exif, coordinate_tag: Tag, reference_tag: Tag) -> Option<f64> {
    let field = exif.get_field(coordinate_tag, In::PRIMARY)?;
    let Value::Rational(values) = &field.value else {
        return None;
    };
    if values.len() < 3 {
        return None;
    }
    let mut coordinate =
        values[0].to_f64() + values[1].to_f64() / 60.0 + values[2].to_f64() / 3600.0;
    let reference = display_field(exif, reference_tag).unwrap_or_default();
    if reference.starts_with('S') || reference.starts_with('W') {
        coordinate = -coordinate;
    }
    Some(coordinate)
}

pub fn media_kind_for_extension(extension: &str) -> MediaKind {
    match extension.to_ascii_lowercase().as_str() {
        "jpg" | "jpeg" | "png" | "heic" | "webp" => MediaKind::Image,
        "mp4" | "mov" => MediaKind::Video,
        _ => MediaKind::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_returns_filesystem_metadata_for_unknown_files() {
        let metadata =
            ExtractorRegistry::standard().extract(Path::new("notes.txt"), MediaKind::Unknown);
        assert_eq!(metadata.extractor, "filesystem");
    }

    #[test]
    fn supported_media_extensions_are_case_insensitive() {
        assert_eq!(media_kind_for_extension("JPG"), MediaKind::Image);
        assert_eq!(media_kind_for_extension("mov"), MediaKind::Video);
        assert_eq!(media_kind_for_extension("txt"), MediaKind::Unknown);
        assert_eq!(media_kind_for_extension("HEIC"), MediaKind::Image);
        assert_eq!(media_kind_for_extension("webp"), MediaKind::Image);
        assert_eq!(media_kind_for_extension("MP4"), MediaKind::Video);
    }

    #[test]
    fn image_extractor_reads_real_dimensions() {
        let temporary = tempfile::tempdir().unwrap();
        let path = temporary.path().join("fixture.png");
        image::DynamicImage::new_rgb8(17, 23).save(&path).unwrap();
        let metadata = ExtractorRegistry::standard().extract(&path, MediaKind::Image);
        assert_eq!(metadata.width, Some(17));
        assert_eq!(metadata.height, Some(23));
        assert_eq!(metadata.extractor, "image+exif");
    }
}
