//! Annotation import from PDF files
//!
//! Provides functionality to read existing PDF annotations and convert them
//! to the application's annotation format.

use crate::annotation::{
    Annotation, AnnotationGeometry, AnnotationMetadata, AnnotationStyle, Color, PageCoordinate,
    SerializableAnnotation,
};
use pdfium_render::prelude::*;
use std::path::Path;

/// Error types for annotation import operations
#[derive(Debug)]
pub enum AnnotationImportError {
    /// Failed to initialize PDFium library
    InitializationError(String),
    /// Failed to load PDF document
    LoadError(String),
    /// Invalid page index
    InvalidPageIndex(u16),
    /// Failed to read annotation
    ReadError(String),
}

impl std::fmt::Display for AnnotationImportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AnnotationImportError::InitializationError(msg) => {
                write!(f, "PDFium initialization error: {}", msg)
            }
            AnnotationImportError::LoadError(msg) => write!(f, "PDF load error: {}", msg),
            AnnotationImportError::InvalidPageIndex(idx) => write!(f, "Invalid page index: {}", idx),
            AnnotationImportError::ReadError(msg) => write!(f, "Annotation read error: {}", msg),
        }
    }
}

impl std::error::Error for AnnotationImportError {}

/// Result type for annotation import operations
pub type AnnotationImportResult<T> = Result<T, AnnotationImportError>;

/// Statistics about imported annotations
#[derive(Debug, Clone, Default)]
pub struct ImportStats {
    /// Total annotations found in PDF
    pub total_found: usize,
    /// Annotations successfully imported
    pub imported: usize,
    /// Annotations skipped (unsupported types)
    pub skipped: usize,
    /// Count by annotation type
    pub by_type: std::collections::HashMap<String, usize>,
}

/// Initialize PDFium library
fn init_pdfium() -> AnnotationImportResult<Pdfium> {
    // Get the executable's directory for app bundle support
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()));

    // Try executable directory first (app bundle support)
    if let Some(ref dir) = exe_dir {
        if let Ok(bindings) =
            Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path(dir))
        {
            return Ok(Pdfium::new(bindings));
        }
    }

    // Fall back to current directory and system library
    Ok(Pdfium::new(
        Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path("./"))
            .or_else(|_| Pdfium::bind_to_system_library())
            .map_err(|e| AnnotationImportError::InitializationError(e.to_string()))?,
    ))
}

/// Convert PDFium color to our Color type
fn convert_color(pdf_color: &PdfColor) -> Color {
    Color::new(
        pdf_color.red(),
        pdf_color.green(),
        pdf_color.blue(),
        pdf_color.alpha(),
    )
}

/// Convert PDFium rectangle bounds to our coordinate system
fn convert_bounds(bounds: &PdfRect) -> (PageCoordinate, PageCoordinate) {
    // PDFium uses bottom-left origin, same as our PDF coordinate system
    let top_left = PageCoordinate::new(bounds.left().value, bounds.top().value);
    let bottom_right = PageCoordinate::new(bounds.right().value, bounds.bottom().value);
    (top_left, bottom_right)
}

/// Parse PDF date string to Unix timestamp
fn parse_pdf_date(date_str: &str) -> i64 {
    // PDF dates are in format: D:YYYYMMDDHHmmSSOHH'mm'
    // For simplicity, we'll try to extract the basic components
    if date_str.starts_with("D:") && date_str.len() >= 10 {
        let date_part = &date_str[2..];
        if let (Ok(year), Ok(month), Ok(day)) = (
            date_part.get(0..4).unwrap_or("1970").parse::<i64>(),
            date_part.get(4..6).unwrap_or("01").parse::<i64>(),
            date_part.get(6..8).unwrap_or("01").parse::<i64>(),
        ) {
            // Simple approximation: days since epoch
            // (This is a rough estimate, not accounting for leap years properly)
            let days_since_epoch = (year - 1970) * 365 + (month - 1) * 30 + day;
            return days_since_epoch * 86400;
        }
    }
    // Return current time if parsing fails
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

/// Convert a single PDF annotation to our format
fn convert_annotation(
    annotation: &PdfPageAnnotation,
    page_index: u16,
) -> Option<SerializableAnnotation> {
    use PdfPageAnnotation::*;

    // Validate bounds exist (if annotation has no bounds, skip it)
    let _bounds = annotation.bounds().ok()?;

    // Get colors with fallbacks
    let stroke_color = annotation
        .stroke_color()
        .ok()
        .map(|c| convert_color(&c))
        .unwrap_or(Color::BLACK);

    let fill_color = annotation.fill_color().ok().map(|c| convert_color(&c));

    // Build style
    let style = AnnotationStyle {
        stroke_color,
        stroke_width: 2.0, // Default stroke width
        fill_color,
        dash_pattern: Vec::new(),
        opacity: 1.0,
        font_size: 12.0,
        font_family: "Helvetica".to_string(),
    };

    // Build metadata
    let mut metadata = AnnotationMetadata::new();
    metadata.label = annotation.contents();
    metadata.author = annotation.creator();

    if let Some(date_str) = annotation.creation_date() {
        metadata.created_at = parse_pdf_date(&date_str);
    }
    if let Some(date_str) = annotation.modification_date() {
        metadata.modified_at = parse_pdf_date(&date_str);
    }

    // Convert geometry based on annotation type
    let geometry = match annotation {
        Highlight(annot) => {
            // Highlight annotations are rectangles over text
            let bounds = annot.bounds().ok()?;
            let (tl, br) = convert_bounds(&bounds);
            Some(AnnotationGeometry::Rectangle {
                top_left: tl,
                bottom_right: br,
            })
        }

        Underline(annot) => {
            // Underline is a line under text
            let bounds = annot.bounds().ok()?;
            let (tl, br) = convert_bounds(&bounds);
            Some(AnnotationGeometry::Line {
                start: PageCoordinate::new(tl.x, br.y),
                end: PageCoordinate::new(br.x, br.y),
            })
        }

        Strikeout(annot) => {
            // Strikeout is a line through text (middle)
            let bounds = annot.bounds().ok()?;
            let (tl, br) = convert_bounds(&bounds);
            let mid_y = (tl.y + br.y) / 2.0;
            Some(AnnotationGeometry::Line {
                start: PageCoordinate::new(tl.x, mid_y),
                end: PageCoordinate::new(br.x, mid_y),
            })
        }

        Squiggly(annot) => {
            // Squiggly is similar to underline but wavy - we represent as line
            let bounds = annot.bounds().ok()?;
            let (tl, br) = convert_bounds(&bounds);
            Some(AnnotationGeometry::Line {
                start: PageCoordinate::new(tl.x, br.y),
                end: PageCoordinate::new(br.x, br.y),
            })
        }

        Text(annot) => {
            // Text annotations are sticky notes at a point
            let bounds = annot.bounds().ok()?;
            let center_x = (bounds.left().value + bounds.right().value) / 2.0;
            let center_y = (bounds.top().value + bounds.bottom().value) / 2.0;
            Some(AnnotationGeometry::Note {
                position: PageCoordinate::new(center_x, center_y),
                icon_size: 24.0,
            })
        }

        FreeText(annot) => {
            // Free text annotations are text boxes
            let bounds = annot.bounds().ok()?;
            Some(AnnotationGeometry::Text {
                position: PageCoordinate::new(bounds.left().value, bounds.top().value),
                max_width: Some(bounds.right().value - bounds.left().value),
            })
        }

        Square(annot) => {
            // Square/Rectangle annotation
            let bounds = annot.bounds().ok()?;
            let (tl, br) = convert_bounds(&bounds);
            Some(AnnotationGeometry::Rectangle {
                top_left: tl,
                bottom_right: br,
            })
        }

        Circle(annot) => {
            // Circle/Ellipse annotation
            let bounds = annot.bounds().ok()?;
            let center_x = (bounds.left().value + bounds.right().value) / 2.0;
            let center_y = (bounds.top().value + bounds.bottom().value) / 2.0;
            let radius_x = (bounds.right().value - bounds.left().value) / 2.0;
            let radius_y = (bounds.top().value - bounds.bottom().value) / 2.0;

            if (radius_x - radius_y).abs() < 0.1 {
                // It's a circle
                Some(AnnotationGeometry::Circle {
                    center: PageCoordinate::new(center_x, center_y),
                    radius: radius_x,
                })
            } else {
                // It's an ellipse
                Some(AnnotationGeometry::Ellipse {
                    center: PageCoordinate::new(center_x, center_y),
                    radius_x,
                    radius_y,
                })
            }
        }

        Ink(annot) => {
            // Ink annotations are freehand drawings
            // PDFium stores ink as path objects - we extract points from bounds
            // as a simplified representation
            let bounds = annot.bounds().ok()?;
            let (tl, br) = convert_bounds(&bounds);

            // For ink annotations without explicit path data,
            // create a simple diagonal line within bounds
            Some(AnnotationGeometry::Freehand {
                points: vec![tl, br],
            })
        }

        // Unsupported annotation types - these are either interactive elements
        // or not useful for editing
        Link(_)
        | Popup(_)
        | Widget(_)
        | XfaWidget(_)
        | Redacted(_)
        | Stamp(_)
        | Unsupported(_) => None,
    }?;

    Some(SerializableAnnotation {
        id: uuid::Uuid::new_v4(),
        page_index,
        geometry,
        style,
        metadata,
        visible: !annotation.is_hidden(),
        layer: 0,
    })
}

/// Get annotation type name for statistics
fn annotation_type_name(annotation: &PdfPageAnnotation) -> &'static str {
    use PdfPageAnnotation::*;
    match annotation {
        Highlight(_) => "Highlight",
        Underline(_) => "Underline",
        Strikeout(_) => "Strikeout",
        Squiggly(_) => "Squiggly",
        Text(_) => "Text",
        FreeText(_) => "FreeText",
        Square(_) => "Square",
        Circle(_) => "Circle",
        Ink(_) => "Ink",
        Link(_) => "Link",
        Popup(_) => "Popup",
        Widget(_) => "Widget",
        XfaWidget(_) => "XfaWidget",
        Redacted(_) => "Redacted",
        Stamp(_) => "Stamp",
        Unsupported(_) => "Unsupported",
    }
}

/// Load annotations from a single page
pub fn load_annotations_from_page(
    pdf_path: &Path,
    page_index: u16,
) -> AnnotationImportResult<(Vec<SerializableAnnotation>, ImportStats)> {
    let pdfium = Box::leak(Box::new(init_pdfium()?));

    let document = pdfium
        .load_pdf_from_file(pdf_path, None)
        .map_err(|e| AnnotationImportError::LoadError(e.to_string()))?;

    let page = document
        .pages()
        .get(page_index)
        .map_err(|_| AnnotationImportError::InvalidPageIndex(page_index))?;

    let mut annotations = Vec::new();
    let mut stats = ImportStats::default();

    for annotation in page.annotations().iter() {
        stats.total_found += 1;
        let type_name = annotation_type_name(&annotation);
        *stats.by_type.entry(type_name.to_string()).or_insert(0) += 1;

        if let Some(converted) = convert_annotation(&annotation, page_index) {
            annotations.push(converted);
            stats.imported += 1;
        } else {
            stats.skipped += 1;
        }
    }

    Ok((annotations, stats))
}

/// Load all annotations from a PDF document
pub fn load_annotations_from_pdf(
    pdf_path: &Path,
) -> AnnotationImportResult<(Vec<SerializableAnnotation>, ImportStats)> {
    let pdfium = Box::leak(Box::new(init_pdfium()?));

    let document = pdfium
        .load_pdf_from_file(pdf_path, None)
        .map_err(|e| AnnotationImportError::LoadError(e.to_string()))?;

    let mut all_annotations = Vec::new();
    let mut total_stats = ImportStats::default();

    let page_count = document.pages().len();

    for page_index in 0..page_count {
        let page = document
            .pages()
            .get(page_index)
            .map_err(|_| AnnotationImportError::InvalidPageIndex(page_index))?;

        for annotation in page.annotations().iter() {
            total_stats.total_found += 1;
            let type_name = annotation_type_name(&annotation);
            *total_stats
                .by_type
                .entry(type_name.to_string())
                .or_insert(0) += 1;

            if let Some(converted) = convert_annotation(&annotation, page_index) {
                all_annotations.push(converted);
                total_stats.imported += 1;
            } else {
                total_stats.skipped += 1;
            }
        }
    }

    Ok((all_annotations, total_stats))
}

/// Load annotations and convert them to Annotation objects
pub fn import_annotations(pdf_path: &Path) -> AnnotationImportResult<Vec<Annotation>> {
    let (serializable, _stats) = load_annotations_from_pdf(pdf_path)?;
    Ok(serializable.into_iter().map(Annotation::from).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_convert_color() {
        let pdf_color = PdfColor::new(255, 128, 64, 200);
        let color = convert_color(&pdf_color);
        assert_eq!(color.r, 255);
        assert_eq!(color.g, 128);
        assert_eq!(color.b, 64);
        assert_eq!(color.a, 200);
    }

    #[test]
    fn test_parse_pdf_date_valid() {
        let date_str = "D:20240115120000";
        let timestamp = parse_pdf_date(date_str);
        // Should be sometime in 2024
        assert!(timestamp > 1700000000); // After Nov 2023
    }

    #[test]
    fn test_parse_pdf_date_invalid() {
        let date_str = "invalid";
        let timestamp = parse_pdf_date(date_str);
        // Should return current time (roughly)
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        assert!((timestamp - now).abs() < 5); // Within 5 seconds
    }


    #[test]
    fn test_import_stats_default() {
        let stats = ImportStats::default();
        assert_eq!(stats.total_found, 0);
        assert_eq!(stats.imported, 0);
        assert_eq!(stats.skipped, 0);
        assert!(stats.by_type.is_empty());
    }

    #[test]
    fn test_error_display() {
        let err = AnnotationImportError::InvalidPageIndex(5);
        assert_eq!(err.to_string(), "Invalid page index: 5");

        let err = AnnotationImportError::LoadError("file not found".to_string());
        assert!(err.to_string().contains("file not found"));

        let err = AnnotationImportError::InitializationError("init failed".to_string());
        assert!(err.to_string().contains("init failed"));

        let err = AnnotationImportError::ReadError("read failed".to_string());
        assert!(err.to_string().contains("read failed"));
    }
}
