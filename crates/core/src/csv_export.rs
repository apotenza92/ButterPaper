//! CSV export for markups (annotations) and measurements
//!
//! Provides functionality to export annotations and measurements to CSV format
//! for analysis, reporting, and integration with external tools.

use crate::annotation::{Annotation, AnnotationGeometry, Color};
use crate::measurement::{Measurement, MeasurementCollection, MeasurementType, ScaleSystem};
use std::io::Write;

/// Error types for CSV export
#[derive(Debug, thiserror::Error)]
pub enum CsvExportError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("CSV serialization error: {0}")]
    Csv(#[from] csv::Error),

    #[error("Scale system not found for measurement: {0}")]
    ScaleNotFound(String),
}

pub type CsvExportResult<T> = Result<T, CsvExportError>;

/// Configuration for CSV export
#[derive(Debug, Clone)]
pub struct CsvExportConfig {
    /// Include column headers in the output
    pub include_headers: bool,

    /// CSV delimiter character
    pub delimiter: u8,

    /// Include hidden items in export
    pub include_hidden: bool,

    /// Export only items from specific pages (None = all pages)
    pub page_filter: Option<Vec<u16>>,
}

impl Default for CsvExportConfig {
    fn default() -> Self {
        Self {
            include_headers: true,
            delimiter: b',',
            include_hidden: false,
            page_filter: None,
        }
    }
}

/// Export annotations to CSV format
///
/// CSV columns:
/// - ID: Unique annotation identifier
/// - Page: Page index (0-based)
/// - Type: Geometry type (Line, Rectangle, Circle, etc.)
/// - Label: User-provided label (if any)
/// - Author: Author name (if any)
/// - Created: Creation timestamp (Unix seconds)
/// - Modified: Last modification timestamp (Unix seconds)
/// - Tags: Comma-separated tags
/// - Stroke Color: Hex color code for stroke (e.g., #FF0000)
/// - Fill Color: Hex color code for fill (e.g., #FFFF00) or empty if no fill
/// - Stroke Width: Line width in points
/// - Geometry: Serialized geometry data (JSON-like format)
/// - BBox Min X: Minimum X coordinate of bounding box
/// - BBox Min Y: Minimum Y coordinate of bounding box
/// - BBox Max X: Maximum X coordinate of bounding box
/// - BBox Max Y: Maximum Y coordinate of bounding box
/// - Visible: Whether annotation is visible (true/false)
/// - Layer: Z-order layer number
pub fn export_annotations_csv<W: Write>(
    writer: W,
    annotations: &[&Annotation],
    config: &CsvExportConfig,
) -> CsvExportResult<()> {
    let mut csv_writer = csv::WriterBuilder::new()
        .delimiter(config.delimiter)
        .has_headers(config.include_headers)
        .from_writer(writer);

    // Write headers
    if config.include_headers {
        csv_writer.write_record([
            "ID",
            "Page",
            "Type",
            "Label",
            "Author",
            "Created",
            "Modified",
            "Tags",
            "Stroke Color",
            "Fill Color",
            "Stroke Width",
            "Geometry",
            "BBox Min X",
            "BBox Min Y",
            "BBox Max X",
            "BBox Max Y",
            "Visible",
            "Layer",
        ])?;
    }

    // Filter annotations based on configuration
    let filtered_annotations: Vec<&Annotation> = annotations
        .iter()
        .copied()
        .filter(|a| {
            // Filter by visibility
            if !config.include_hidden && !a.is_visible() {
                return false;
            }

            // Filter by page
            if let Some(ref pages) = config.page_filter {
                if !pages.contains(&a.page_index()) {
                    return false;
                }
            }

            true
        })
        .collect();

    // Write annotation rows
    for annotation in filtered_annotations {
        let metadata = annotation.metadata();
        let style = annotation.style();
        let geometry = annotation.geometry();
        let (bbox_min_x, bbox_min_y, bbox_max_x, bbox_max_y) = annotation.bounding_box();

        csv_writer.write_record(&[
            annotation.id().to_string(),
            annotation.page_index().to_string(),
            geometry_type_name(geometry),
            metadata.label.as_deref().unwrap_or("").to_string(),
            metadata.author.as_deref().unwrap_or("").to_string(),
            metadata.created_at.to_string(),
            metadata.modified_at.to_string(),
            metadata.tags.join(";"),
            color_to_hex(&style.stroke_color),
            style
                .fill_color
                .as_ref()
                .map(color_to_hex)
                .unwrap_or_default(),
            style.stroke_width.to_string(),
            format_geometry(geometry),
            bbox_min_x.to_string(),
            bbox_min_y.to_string(),
            bbox_max_x.to_string(),
            bbox_max_y.to_string(),
            annotation.is_visible().to_string(),
            annotation.layer().to_string(),
        ])?;
    }

    csv_writer.flush()?;
    Ok(())
}

/// Export measurements to CSV format
///
/// CSV columns:
/// - ID: Unique measurement identifier
/// - Page: Page index (0-based)
/// - Type: Measurement type (Distance, Area, Radius, Angle)
/// - Value: Computed measurement value in real-world units
/// - Unit: Unit of measurement (e.g., "m", "ft", "inches")
/// - Formatted: Formatted label with value and unit
/// - Scale System ID: ID of the scale system used
/// - Scale Ratio: Scale ratio (page units per real-world unit)
/// - Label: User-provided label (if any)
/// - Tags: Comma-separated tags
/// - Notes: User notes (if any)
/// - Geometry: Serialized geometry data
/// - Label Position X: X coordinate where label should be placed
/// - Label Position Y: Y coordinate where label should be placed
/// - BBox Min X: Minimum X coordinate of bounding box
/// - BBox Min Y: Minimum Y coordinate of bounding box
/// - BBox Max X: Maximum X coordinate of bounding box
/// - BBox Max Y: Maximum Y coordinate of bounding box
/// - Visible: Whether measurement is visible (true/false)
/// - Layer: Z-order layer number
pub fn export_measurements_csv<W: Write>(
    writer: W,
    collection: &MeasurementCollection,
    config: &CsvExportConfig,
) -> CsvExportResult<()> {
    let mut csv_writer = csv::WriterBuilder::new()
        .delimiter(config.delimiter)
        .has_headers(config.include_headers)
        .from_writer(writer);

    // Write headers
    if config.include_headers {
        csv_writer.write_record([
            "ID",
            "Page",
            "Type",
            "Value",
            "Unit",
            "Formatted",
            "Scale System ID",
            "Scale Ratio",
            "Label",
            "Tags",
            "Notes",
            "Geometry",
            "Label Position X",
            "Label Position Y",
            "BBox Min X",
            "BBox Min Y",
            "BBox Max X",
            "BBox Max Y",
            "Visible",
            "Layer",
        ])?;
    }

    // Collect all measurements
    let mut all_measurements: Vec<&Measurement> = Vec::new();

    if let Some(ref pages) = config.page_filter {
        for &page in pages {
            all_measurements.extend(collection.get_for_page(page));
        }
    } else {
        // Get measurements from all pages
        // Since we don't have a direct method to get all measurements,
        // we'll need to iterate through potential pages
        // For now, we'll collect measurements from pages 0-65535
        for page_idx in 0..=u16::MAX {
            let page_measurements = collection.get_for_page(page_idx);
            if page_measurements.is_empty() {
                continue;
            }
            all_measurements.extend(page_measurements);
        }
    }

    // Filter by visibility
    let filtered_measurements: Vec<&Measurement> = all_measurements
        .into_iter()
        .filter(|m| config.include_hidden || m.is_visible())
        .collect();

    // Write measurement rows
    for measurement in filtered_measurements {
        let metadata = measurement.metadata();
        let geometry = measurement.geometry();
        let label_pos = measurement.label_position();
        let (bbox_min_x, bbox_min_y, bbox_max_x, bbox_max_y) = geometry.bounding_box();

        // Get scale system information
        let scale = collection.get_scale(measurement.scale_system_id());
        let unit = scale.map(|s| s.unit()).unwrap_or("");
        let ratio = scale.map(|s| s.ratio()).unwrap_or(1.0);

        csv_writer.write_record(&[
            measurement.id().to_string(),
            measurement.page_index().to_string(),
            measurement_type_name(measurement.measurement_type()),
            measurement
                .value()
                .map(|v| v.to_string())
                .unwrap_or_default(),
            unit.to_string(),
            measurement.formatted_label().unwrap_or("").to_string(),
            measurement.scale_system_id().to_string(),
            ratio.to_string(),
            metadata.label.as_deref().unwrap_or("").to_string(),
            metadata.tags.join(";"),
            metadata.notes.as_deref().unwrap_or("").to_string(),
            format_geometry(geometry),
            label_pos.x.to_string(),
            label_pos.y.to_string(),
            bbox_min_x.to_string(),
            bbox_min_y.to_string(),
            bbox_max_x.to_string(),
            bbox_max_y.to_string(),
            measurement.is_visible().to_string(),
            measurement.layer().to_string(),
        ])?;
    }

    csv_writer.flush()?;
    Ok(())
}

/// Export scale systems to CSV format
///
/// CSV columns:
/// - ID: Unique scale system identifier
/// - Page: Page index (0-based)
/// - Type: Scale type (Manual, TwoPoint, OCRDetected)
/// - Ratio: Scale ratio (page units per real-world unit)
/// - Unit: Unit of measurement
/// - Label: Optional label for the scale system
/// - Reliable: Whether this scale is considered reliable
pub fn export_scales_csv<W: Write>(
    writer: W,
    scales: &[&ScaleSystem],
    config: &CsvExportConfig,
) -> CsvExportResult<()> {
    let mut csv_writer = csv::WriterBuilder::new()
        .delimiter(config.delimiter)
        .has_headers(config.include_headers)
        .from_writer(writer);

    // Write headers
    if config.include_headers {
        csv_writer.write_record(["ID", "Page", "Type", "Ratio", "Unit", "Label", "Reliable"])?;
    }

    // Filter scales based on page filter
    let filtered_scales: Vec<&ScaleSystem> = scales
        .iter()
        .copied()
        .filter(|s| {
            if let Some(ref pages) = config.page_filter {
                pages.contains(&s.page_index())
            } else {
                true
            }
        })
        .collect();

    // Write scale rows
    for scale in filtered_scales {
        csv_writer.write_record(&[
            scale.id().to_string(),
            scale.page_index().to_string(),
            "Scale".to_string(), // Could expand this to show Manual/TwoPoint/OCR
            scale.ratio().to_string(),
            scale.unit().to_string(),
            scale.label().unwrap_or("").to_string(),
            scale.is_reliable().to_string(),
        ])?;
    }

    csv_writer.flush()?;
    Ok(())
}

/// Helper function to get geometry type name
fn geometry_type_name(geometry: &AnnotationGeometry) -> String {
    match geometry {
        AnnotationGeometry::Line { .. } => "Line",
        AnnotationGeometry::Polyline { .. } => "Polyline",
        AnnotationGeometry::Polygon { .. } => "Polygon",
        AnnotationGeometry::Rectangle { .. } => "Rectangle",
        AnnotationGeometry::Circle { .. } => "Circle",
        AnnotationGeometry::Ellipse { .. } => "Ellipse",
        AnnotationGeometry::Freehand { .. } => "Freehand",
        AnnotationGeometry::Text { .. } => "Text",
        AnnotationGeometry::Arrow { .. } => "Arrow",
        AnnotationGeometry::Note { .. } => "Note",
    }
    .to_string()
}

/// Helper function to get measurement type name
fn measurement_type_name(measurement_type: MeasurementType) -> String {
    match measurement_type {
        MeasurementType::Distance => "Distance",
        MeasurementType::Area => "Area",
        MeasurementType::Radius => "Radius",
        MeasurementType::Angle => "Angle",
    }
    .to_string()
}

/// Convert color to hex string (e.g., #FF0000)
fn color_to_hex(color: &Color) -> String {
    format!("#{:02X}{:02X}{:02X}", color.r, color.g, color.b)
}

/// Format geometry as a human-readable string
fn format_geometry(geometry: &AnnotationGeometry) -> String {
    match geometry {
        AnnotationGeometry::Line { start, end } => {
            format!(
                "Line[({:.2},{:.2})-({:.2},{:.2})]",
                start.x, start.y, end.x, end.y
            )
        }
        AnnotationGeometry::Arrow { start, end } => {
            format!(
                "Arrow[({:.2},{:.2})-({:.2},{:.2})]",
                start.x, start.y, end.x, end.y
            )
        }
        AnnotationGeometry::Polyline { points } => {
            let points_str = points
                .iter()
                .map(|p| format!("({:.2},{:.2})", p.x, p.y))
                .collect::<Vec<_>>()
                .join(",");
            format!("Polyline[{}]", points_str)
        }
        AnnotationGeometry::Polygon { points } => {
            let points_str = points
                .iter()
                .map(|p| format!("({:.2},{:.2})", p.x, p.y))
                .collect::<Vec<_>>()
                .join(",");
            format!("Polygon[{}]", points_str)
        }
        AnnotationGeometry::Rectangle {
            top_left,
            bottom_right,
        } => {
            format!(
                "Rectangle[({:.2},{:.2})-({:.2},{:.2})]",
                top_left.x, top_left.y, bottom_right.x, bottom_right.y
            )
        }
        AnnotationGeometry::Circle { center, radius } => {
            format!("Circle[({:.2},{:.2}),r={:.2}]", center.x, center.y, radius)
        }
        AnnotationGeometry::Ellipse {
            center,
            radius_x,
            radius_y,
        } => {
            format!(
                "Ellipse[({:.2},{:.2}),rx={:.2},ry={:.2}]",
                center.x, center.y, radius_x, radius_y
            )
        }
        AnnotationGeometry::Freehand { points } => {
            format!("Freehand[{} points]", points.len())
        }
        AnnotationGeometry::Text {
            position,
            max_width,
        } => {
            let width_str = max_width
                .map(|w| format!(",w={:.2}", w))
                .unwrap_or_default();
            format!("Text[({:.2},{:.2}){}]", position.x, position.y, width_str)
        }
        AnnotationGeometry::Note { position, icon_size } => {
            format!("Note[({:.2},{:.2}),size={:.2}]", position.x, position.y, icon_size)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::annotation::{AnnotationStyle, PageCoordinate};

    #[test]
    fn test_export_annotations_csv() {
        let mut output = Vec::new();

        let annotation1 = Annotation::new(
            0,
            AnnotationGeometry::Line {
                start: PageCoordinate::new(0.0, 0.0),
                end: PageCoordinate::new(100.0, 100.0),
            },
            AnnotationStyle::red_markup(),
        );

        let annotation2 = Annotation::new(
            1,
            AnnotationGeometry::Circle {
                center: PageCoordinate::new(50.0, 50.0),
                radius: 25.0,
            },
            AnnotationStyle::new(),
        );

        let annotations = vec![&annotation1, &annotation2];
        let config = CsvExportConfig::default();

        export_annotations_csv(&mut output, &annotations, &config).unwrap();

        let csv_content = String::from_utf8(output).unwrap();
        assert!(csv_content.contains("ID,Page,Type"));
        assert!(csv_content.contains("Line"));
        assert!(csv_content.contains("Circle"));
        assert!(csv_content.contains("#FF0000")); // Red color
    }

    #[test]
    fn test_export_measurements_csv() {
        let mut output = Vec::new();
        let mut collection = MeasurementCollection::new();

        let scale = ScaleSystem::manual(0, 72.0, "inches");
        let scale_id = collection.add_scale(scale);

        let measurement = Measurement::new(
            0,
            AnnotationGeometry::Line {
                start: PageCoordinate::new(0.0, 0.0),
                end: PageCoordinate::new(72.0, 0.0),
            },
            MeasurementType::Distance,
            scale_id,
        );

        collection.add(measurement);

        let config = CsvExportConfig::default();
        export_measurements_csv(&mut output, &collection, &config).unwrap();

        let csv_content = String::from_utf8(output).unwrap();
        assert!(csv_content.contains("ID,Page,Type,Value"));
        assert!(csv_content.contains("Distance"));
        assert!(csv_content.contains("1.00")); // 72 points = 1 inch
        assert!(csv_content.contains("inches"));
    }

    #[test]
    fn test_color_to_hex() {
        assert_eq!(color_to_hex(&Color::RED), "#FF0000");
        assert_eq!(color_to_hex(&Color::GREEN), "#00FF00");
        assert_eq!(color_to_hex(&Color::BLUE), "#0000FF");
        assert_eq!(color_to_hex(&Color::BLACK), "#000000");
        assert_eq!(color_to_hex(&Color::WHITE), "#FFFFFF");
    }

    #[test]
    fn test_geometry_type_name() {
        let line = AnnotationGeometry::Line {
            start: PageCoordinate::new(0.0, 0.0),
            end: PageCoordinate::new(1.0, 1.0),
        };
        assert_eq!(geometry_type_name(&line), "Line");

        let circle = AnnotationGeometry::Circle {
            center: PageCoordinate::new(0.0, 0.0),
            radius: 10.0,
        };
        assert_eq!(geometry_type_name(&circle), "Circle");
    }

    #[test]
    fn test_measurement_type_name() {
        assert_eq!(measurement_type_name(MeasurementType::Distance), "Distance");
        assert_eq!(measurement_type_name(MeasurementType::Area), "Area");
        assert_eq!(measurement_type_name(MeasurementType::Radius), "Radius");
        assert_eq!(measurement_type_name(MeasurementType::Angle), "Angle");
    }

    #[test]
    fn test_format_geometry() {
        let line = AnnotationGeometry::Line {
            start: PageCoordinate::new(0.0, 0.0),
            end: PageCoordinate::new(100.5, 200.75),
        };
        let formatted = format_geometry(&line);
        assert!(formatted.contains("Line"));
        assert!(formatted.contains("100.50"));
        assert!(formatted.contains("200.75"));
    }

    #[test]
    fn test_csv_with_page_filter() {
        let mut output = Vec::new();

        let annotation1 = Annotation::new(
            0,
            AnnotationGeometry::Line {
                start: PageCoordinate::new(0.0, 0.0),
                end: PageCoordinate::new(100.0, 100.0),
            },
            AnnotationStyle::new(),
        );

        let annotation2 = Annotation::new(
            1,
            AnnotationGeometry::Circle {
                center: PageCoordinate::new(50.0, 50.0),
                radius: 25.0,
            },
            AnnotationStyle::new(),
        );

        let annotations = vec![&annotation1, &annotation2];
        let config = CsvExportConfig {
            page_filter: Some(vec![0]), // Only export page 0
            ..Default::default()
        };

        export_annotations_csv(&mut output, &annotations, &config).unwrap();

        let csv_content = String::from_utf8(output).unwrap();
        let line_count = csv_content.lines().count();
        assert_eq!(line_count, 2); // Header + 1 annotation
    }

    #[test]
    fn test_csv_include_hidden() {
        let mut annotation = Annotation::new(
            0,
            AnnotationGeometry::Line {
                start: PageCoordinate::new(0.0, 0.0),
                end: PageCoordinate::new(100.0, 100.0),
            },
            AnnotationStyle::new(),
        );
        annotation.set_visible(false);

        let annotations = vec![&annotation];

        // Without include_hidden
        let mut output = Vec::new();
        let config = CsvExportConfig {
            include_hidden: false,
            ..Default::default()
        };
        export_annotations_csv(&mut output, &annotations, &config).unwrap();
        let csv_content = String::from_utf8(output).unwrap();
        let line_count = csv_content.lines().count();
        assert_eq!(line_count, 1); // Only header

        // With include_hidden
        let mut output = Vec::new();
        let config = CsvExportConfig {
            include_hidden: true,
            ..Default::default()
        };
        export_annotations_csv(&mut output, &annotations, &config).unwrap();
        let csv_content = String::from_utf8(output).unwrap();
        let line_count = csv_content.lines().count();
        assert_eq!(line_count, 2); // Header + 1 annotation
    }
}
