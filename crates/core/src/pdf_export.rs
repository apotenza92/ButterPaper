//! PDF export with annotations and appearance streams
//!
//! Provides functionality to export PDF documents with standard annotations
//! and generate appearance streams for visual rendering.

use crate::annotation::{AnnotationGeometry, Color, PageCoordinate, SerializableAnnotation};
use crate::document::DocumentMetadata;
use pdfium_render::prelude::*;
use std::fmt::Write as FmtWrite;
use std::path::Path;

/// Error types for PDF export operations
#[derive(Debug)]
pub enum PdfExportError {
    /// IO error during file operations
    IoError(std::io::Error),
    /// PDF generation error
    GenerationError(String),
    /// PDF loading error
    LoadError(String),
    /// Unsupported annotation type
    UnsupportedAnnotation(String),
}

impl std::fmt::Display for PdfExportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PdfExportError::IoError(e) => write!(f, "IO error: {}", e),
            PdfExportError::GenerationError(e) => write!(f, "PDF generation error: {}", e),
            PdfExportError::LoadError(e) => write!(f, "PDF load error: {}", e),
            PdfExportError::UnsupportedAnnotation(e) => write!(f, "Unsupported annotation: {}", e),
        }
    }
}

impl std::error::Error for PdfExportError {}

impl From<std::io::Error> for PdfExportError {
    fn from(err: std::io::Error) -> Self {
        PdfExportError::IoError(err)
    }
}

/// Result type for PDF export operations
pub type PdfExportResult<T> = Result<T, PdfExportError>;

/// PDF coordinate conversion
///
/// PDF coordinate system has origin at bottom-left, matching our PageCoordinate system
fn to_pdf_coord(coord: &PageCoordinate) -> (f32, f32) {
    (coord.x, coord.y)
}

/// Convert color to PDF color space (normalized 0-1 range)
fn to_pdf_color(color: &Color) -> (f32, f32, f32) {
    let (r, g, b, _) = color.to_normalized();
    (r, g, b)
}

/// Generate appearance stream content for an annotation
///
/// Appearance streams are PDF content streams that define how annotations
/// are rendered when displayed or printed.
pub fn generate_appearance_stream(annotation: &SerializableAnnotation) -> PdfExportResult<String> {
    let mut stream = String::new();

    // Set graphics state
    let style = &annotation.style;
    let (r, g, b) = to_pdf_color(&style.stroke_color);

    // Set stroke color
    writeln!(&mut stream, "{} {} {} RG", r, g, b)
        .map_err(|e| PdfExportError::GenerationError(e.to_string()))?;

    // Set line width
    writeln!(&mut stream, "{} w", style.stroke_width)
        .map_err(|e| PdfExportError::GenerationError(e.to_string()))?;

    // Set opacity if not fully opaque
    if style.opacity < 1.0 {
        writeln!(&mut stream, "/GS1 gs")
            .map_err(|e| PdfExportError::GenerationError(e.to_string()))?;
    }

    // Set dash pattern if present
    if !style.dash_pattern.is_empty() {
        write!(&mut stream, "[")
            .map_err(|e| PdfExportError::GenerationError(e.to_string()))?;
        for dash in &style.dash_pattern {
            write!(&mut stream, "{} ", dash)
                .map_err(|e| PdfExportError::GenerationError(e.to_string()))?;
        }
        writeln!(&mut stream, "] 0 d")
            .map_err(|e| PdfExportError::GenerationError(e.to_string()))?;
    }

    // Set fill color if present
    if let Some(fill_color) = &style.fill_color {
        let (r, g, b) = to_pdf_color(fill_color);
        writeln!(&mut stream, "{} {} {} rg", r, g, b)
            .map_err(|e| PdfExportError::GenerationError(e.to_string()))?;
    }

    // Generate geometry-specific path operations
    match &annotation.geometry {
        AnnotationGeometry::Line { start, end } => {
            let (x1, y1) = to_pdf_coord(start);
            let (x2, y2) = to_pdf_coord(end);
            writeln!(&mut stream, "{} {} m", x1, y1)
                .map_err(|e| PdfExportError::GenerationError(e.to_string()))?;
            writeln!(&mut stream, "{} {} l", x2, y2)
                .map_err(|e| PdfExportError::GenerationError(e.to_string()))?;
            writeln!(&mut stream, "S")
                .map_err(|e| PdfExportError::GenerationError(e.to_string()))?;
        }

        AnnotationGeometry::Polyline { points } | AnnotationGeometry::Freehand { points } => {
            if points.is_empty() {
                return Ok(stream);
            }
            let (x, y) = to_pdf_coord(&points[0]);
            writeln!(&mut stream, "{} {} m", x, y)
                .map_err(|e| PdfExportError::GenerationError(e.to_string()))?;

            for point in points.iter().skip(1) {
                let (x, y) = to_pdf_coord(point);
                writeln!(&mut stream, "{} {} l", x, y)
                    .map_err(|e| PdfExportError::GenerationError(e.to_string()))?;
            }
            writeln!(&mut stream, "S")
                .map_err(|e| PdfExportError::GenerationError(e.to_string()))?;
        }

        AnnotationGeometry::Polygon { points } => {
            if points.is_empty() {
                return Ok(stream);
            }
            let (x, y) = to_pdf_coord(&points[0]);
            writeln!(&mut stream, "{} {} m", x, y)
                .map_err(|e| PdfExportError::GenerationError(e.to_string()))?;

            for point in points.iter().skip(1) {
                let (x, y) = to_pdf_coord(point);
                writeln!(&mut stream, "{} {} l", x, y)
                    .map_err(|e| PdfExportError::GenerationError(e.to_string()))?;
            }

            // Close path and fill/stroke
            if style.fill_color.is_some() {
                writeln!(&mut stream, "b") // Close, fill, and stroke
                    .map_err(|e| PdfExportError::GenerationError(e.to_string()))?;
            } else {
                writeln!(&mut stream, "s") // Close and stroke
                    .map_err(|e| PdfExportError::GenerationError(e.to_string()))?;
            }
        }

        AnnotationGeometry::Rectangle { top_left, bottom_right } => {
            let (x, y) = to_pdf_coord(top_left);
            let width = bottom_right.x - top_left.x;
            let height = bottom_right.y - top_left.y;

            writeln!(&mut stream, "{} {} {} {} re", x, y, width, height)
                .map_err(|e| PdfExportError::GenerationError(e.to_string()))?;

            if style.fill_color.is_some() {
                writeln!(&mut stream, "B") // Fill and stroke
                    .map_err(|e| PdfExportError::GenerationError(e.to_string()))?;
            } else {
                writeln!(&mut stream, "S") // Stroke only
                    .map_err(|e| PdfExportError::GenerationError(e.to_string()))?;
            }
        }

        AnnotationGeometry::Circle { center, radius } => {
            // Approximate circle with Bezier curves (4 curves for circle)
            let kappa = 0.552_284_8; // Magic number for circle approximation
            let (cx, cy) = to_pdf_coord(center);
            let r = *radius;
            let k = r * kappa;

            // Start at right point
            writeln!(&mut stream, "{} {} m", cx + r, cy)
                .map_err(|e| PdfExportError::GenerationError(e.to_string()))?;

            // Top right curve
            writeln!(&mut stream, "{} {} {} {} {} {} c",
                cx + r, cy + k, cx + k, cy + r, cx, cy + r)
                .map_err(|e| PdfExportError::GenerationError(e.to_string()))?;

            // Top left curve
            writeln!(&mut stream, "{} {} {} {} {} {} c",
                cx - k, cy + r, cx - r, cy + k, cx - r, cy)
                .map_err(|e| PdfExportError::GenerationError(e.to_string()))?;

            // Bottom left curve
            writeln!(&mut stream, "{} {} {} {} {} {} c",
                cx - r, cy - k, cx - k, cy - r, cx, cy - r)
                .map_err(|e| PdfExportError::GenerationError(e.to_string()))?;

            // Bottom right curve
            writeln!(&mut stream, "{} {} {} {} {} {} c",
                cx + k, cy - r, cx + r, cy - k, cx + r, cy)
                .map_err(|e| PdfExportError::GenerationError(e.to_string()))?;

            if style.fill_color.is_some() {
                writeln!(&mut stream, "b") // Close, fill, and stroke
                    .map_err(|e| PdfExportError::GenerationError(e.to_string()))?;
            } else {
                writeln!(&mut stream, "s") // Close and stroke
                    .map_err(|e| PdfExportError::GenerationError(e.to_string()))?;
            }
        }

        AnnotationGeometry::Ellipse { center, radius_x, radius_y } => {
            // Approximate ellipse with Bezier curves
            let kappa = 0.552_284_8;
            let (cx, cy) = to_pdf_coord(center);
            let rx = *radius_x;
            let ry = *radius_y;
            let kx = rx * kappa;
            let ky = ry * kappa;

            // Start at right point
            writeln!(&mut stream, "{} {} m", cx + rx, cy)
                .map_err(|e| PdfExportError::GenerationError(e.to_string()))?;

            // Top right curve
            writeln!(&mut stream, "{} {} {} {} {} {} c",
                cx + rx, cy + ky, cx + kx, cy + ry, cx, cy + ry)
                .map_err(|e| PdfExportError::GenerationError(e.to_string()))?;

            // Top left curve
            writeln!(&mut stream, "{} {} {} {} {} {} c",
                cx - kx, cy + ry, cx - rx, cy + ky, cx - rx, cy)
                .map_err(|e| PdfExportError::GenerationError(e.to_string()))?;

            // Bottom left curve
            writeln!(&mut stream, "{} {} {} {} {} {} c",
                cx - rx, cy - ky, cx - kx, cy - ry, cx, cy - ry)
                .map_err(|e| PdfExportError::GenerationError(e.to_string()))?;

            // Bottom right curve
            writeln!(&mut stream, "{} {} {} {} {} {} c",
                cx + kx, cy - ry, cx + rx, cy - ky, cx + rx, cy)
                .map_err(|e| PdfExportError::GenerationError(e.to_string()))?;

            if style.fill_color.is_some() {
                writeln!(&mut stream, "b") // Close, fill, and stroke
                    .map_err(|e| PdfExportError::GenerationError(e.to_string()))?;
            } else {
                writeln!(&mut stream, "s") // Close and stroke
                    .map_err(|e| PdfExportError::GenerationError(e.to_string()))?;
            }
        }

        AnnotationGeometry::Arrow { start, end } => {
            // Draw main line
            let (x1, y1) = to_pdf_coord(start);
            let (x2, y2) = to_pdf_coord(end);
            writeln!(&mut stream, "{} {} m", x1, y1)
                .map_err(|e| PdfExportError::GenerationError(e.to_string()))?;
            writeln!(&mut stream, "{} {} l", x2, y2)
                .map_err(|e| PdfExportError::GenerationError(e.to_string()))?;
            writeln!(&mut stream, "S")
                .map_err(|e| PdfExportError::GenerationError(e.to_string()))?;

            // Draw arrowhead
            let dx = x2 - x1;
            let dy = y2 - y1;
            let angle = dy.atan2(dx);
            let arrow_len = 10.0 * style.stroke_width;
            let arrow_angle = std::f32::consts::PI / 6.0; // 30 degrees

            let x3 = x2 - arrow_len * (angle - arrow_angle).cos();
            let y3 = y2 - arrow_len * (angle - arrow_angle).sin();
            let x4 = x2 - arrow_len * (angle + arrow_angle).cos();
            let y4 = y2 - arrow_len * (angle + arrow_angle).sin();

            writeln!(&mut stream, "{} {} m", x2, y2)
                .map_err(|e| PdfExportError::GenerationError(e.to_string()))?;
            writeln!(&mut stream, "{} {} l", x3, y3)
                .map_err(|e| PdfExportError::GenerationError(e.to_string()))?;
            writeln!(&mut stream, "{} {} m", x2, y2)
                .map_err(|e| PdfExportError::GenerationError(e.to_string()))?;
            writeln!(&mut stream, "{} {} l", x4, y4)
                .map_err(|e| PdfExportError::GenerationError(e.to_string()))?;
            writeln!(&mut stream, "S")
                .map_err(|e| PdfExportError::GenerationError(e.to_string()))?;
        }

        AnnotationGeometry::Text { position, max_width: _ } => {
            // For text annotations, we'll create a simple text appearance
            let (x, y) = to_pdf_coord(position);
            writeln!(&mut stream, "BT")
                .map_err(|e| PdfExportError::GenerationError(e.to_string()))?;
            writeln!(&mut stream, "/{} {} Tf", style.font_family, style.font_size)
                .map_err(|e| PdfExportError::GenerationError(e.to_string()))?;
            writeln!(&mut stream, "{} {} Td", x, y)
                .map_err(|e| PdfExportError::GenerationError(e.to_string()))?;

            // Use label from metadata if available
            if let Some(label) = &annotation.metadata.label {
                // Escape special characters in PDF strings
                let escaped = label.replace('\\', "\\\\")
                    .replace('(', "\\(")
                    .replace(')', "\\)");
                writeln!(&mut stream, "({}) Tj", escaped)
                    .map_err(|e| PdfExportError::GenerationError(e.to_string()))?;
            }

            writeln!(&mut stream, "ET")
                .map_err(|e| PdfExportError::GenerationError(e.to_string()))?;
        }
    }

    Ok(stream)
}

/// Export options for PDF save
#[derive(Debug, Clone)]
pub struct ExportOptions {
    /// Include annotations in export
    pub include_annotations: bool,

    /// Generate appearance streams for annotations
    pub generate_appearances: bool,

    /// Flatten annotations (make them part of page content)
    pub flatten: bool,
}

impl Default for ExportOptions {
    fn default() -> Self {
        Self {
            include_annotations: true,
            generate_appearances: true,
            flatten: false,
        }
    }
}

/// Save PDF with annotations
///
/// This function currently generates appearance streams for annotations
/// and prepares the data for PDF export. The actual PDF modification
/// requires integration with a PDF writer library.
///
/// # Note
/// This is a foundational implementation. Full PDF writing requires:
/// - PDF writer library (lopdf, printpdf, or pdfium write API)
/// - Object graph construction
/// - Annotation dictionary creation
/// - Appearance stream embedding
pub fn save_pdf_with_annotations(
    _source_path: &Path,
    _output_path: &Path,
    metadata: &DocumentMetadata,
    options: &ExportOptions,
) -> PdfExportResult<()> {
    if !options.include_annotations || metadata.annotations.is_empty() {
        return Err(PdfExportError::GenerationError(
            "No annotations to export or annotations disabled".to_string()
        ));
    }

    // Generate appearance streams for all annotations
    for annotation in &metadata.annotations {
        let _appearance_stream = generate_appearance_stream(annotation)?;

        // In a full implementation, this would:
        // 1. Load the source PDF
        // 2. Create annotation dictionaries with:
        //    - /Type /Annot
        //    - /Subtype (Line, Polygon, Square, Circle, FreeText, etc.)
        //    - /Rect (bounding box)
        //    - /C (color)
        //    - /Border (border style)
        //    - /AP (appearance dictionary with /N for normal appearance)
        // 3. Embed appearance streams as form XObjects
        // 4. Add annotations to page annotation array
        // 5. Write modified PDF to output path
    }

    // Placeholder: This would integrate with a PDF writer
    Ok(())
}

/// Convert color to pdfium RGB values (0-255 range)
fn to_pdfium_color(color: &Color) -> (u8, u8, u8) {
    let (r, g, b, _) = color.to_normalized();
    ((r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8)
}

/// Add annotation as a path object to a page
fn add_annotation_to_page<'a>(
    page: &mut PdfPage<'a>,
    annotation: &SerializableAnnotation,
    document: &PdfDocument<'a>,
) -> PdfExportResult<()> {
    let (r, g, b) = to_pdfium_color(&annotation.style.stroke_color);
    let stroke_color = PdfColor::new(r, g, b, 255);
    let stroke_width = PdfPoints::new(annotation.style.stroke_width);

    let fill_color = annotation.style.fill_color.as_ref().map(|c| {
        let (r, g, b) = to_pdfium_color(c);
        PdfColor::new(r, g, b, 255)
    });

    match &annotation.geometry {
        AnnotationGeometry::Line { start, end } => {
            let path = PdfPagePathObject::new_line(
                document,
                PdfPoints::new(start.x),
                PdfPoints::new(start.y),
                PdfPoints::new(end.x),
                PdfPoints::new(end.y),
                stroke_color,
                stroke_width,
            ).map_err(|e| PdfExportError::GenerationError(e.to_string()))?;

            page.objects_mut()
                .add_path_object(path)
                .map_err(|e| PdfExportError::GenerationError(e.to_string()))?;
        }

        AnnotationGeometry::Rectangle { top_left, bottom_right } => {
            // PDF coordinates: bottom-left origin
            let rect = PdfRect::new_from_values(
                top_left.y,        // bottom
                top_left.x,        // left
                bottom_right.y,    // top
                bottom_right.x,    // right
            );

            let obj = page.objects_mut()
                .create_path_object_rect(
                    rect,
                    Some(stroke_color),
                    Some(stroke_width),
                    fill_color,
                )
                .map_err(|e| PdfExportError::GenerationError(e.to_string()))?;

            // Object is already added to the page by create_path_object_rect
            drop(obj);
        }

        AnnotationGeometry::Circle { center, radius } => {
            // Create circle as ellipse with equal radii
            let rect = PdfRect::new_from_values(
                center.y - radius,    // bottom
                center.x - radius,    // left
                center.y + radius,    // top
                center.x + radius,    // right
            );

            let obj = page.objects_mut()
                .create_path_object_ellipse(
                    rect,
                    Some(stroke_color),
                    Some(stroke_width),
                    fill_color,
                )
                .map_err(|e| PdfExportError::GenerationError(e.to_string()))?;

            drop(obj);
        }

        AnnotationGeometry::Ellipse { center, radius_x, radius_y } => {
            let rect = PdfRect::new_from_values(
                center.y - radius_y,    // bottom
                center.x - radius_x,    // left
                center.y + radius_y,    // top
                center.x + radius_x,    // right
            );

            let obj = page.objects_mut()
                .create_path_object_ellipse(
                    rect,
                    Some(stroke_color),
                    Some(stroke_width),
                    fill_color,
                )
                .map_err(|e| PdfExportError::GenerationError(e.to_string()))?;

            drop(obj);
        }

        // For now, complex geometries (polyline, polygon, arrow, text) are not supported
        // in flattened export. These would require more complex path construction.
        AnnotationGeometry::Polyline { .. }
        | AnnotationGeometry::Freehand { .. }
        | AnnotationGeometry::Polygon { .. }
        | AnnotationGeometry::Arrow { .. }
        | AnnotationGeometry::Text { .. } => {
            // Skip complex geometries for now
            return Ok(());
        }
    }

    Ok(())
}

/// Initialize pdfium library
fn init_pdfium() -> PdfExportResult<Pdfium> {
    Ok(Pdfium::new(
        Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path("./"))
            .or_else(|_| Pdfium::bind_to_system_library())
            .map_err(|e| PdfExportError::GenerationError(e.to_string()))?,
    ))
}

/// Export PDF with flattened annotations
///
/// This function renders annotations directly into the page content stream,
/// making them permanent and non-editable. This ensures maximum compatibility
/// with all PDF viewers and prevents annotations from being removed.
///
/// # Arguments
/// * `source_path` - Path to the source PDF file
/// * `output_path` - Path where the flattened PDF will be saved
/// * `metadata` - Document metadata containing annotations
///
/// # Returns
/// Success or an error if the operation fails
pub fn export_flattened_pdf(
    source_path: &Path,
    output_path: &Path,
    metadata: &DocumentMetadata,
) -> PdfExportResult<()> {
    if metadata.annotations.is_empty() {
        return Err(PdfExportError::GenerationError(
            "No annotations to flatten".to_string()
        ));
    }

    // Initialize pdfium
    let pdfium = init_pdfium()?;

    // Load the source PDF
    let mut document = pdfium
        .load_pdf_from_file(source_path, None)
        .map_err(|e| PdfExportError::LoadError(e.to_string()))?;

    // Group annotations by page
    let mut annotations_by_page: std::collections::HashMap<usize, Vec<&SerializableAnnotation>> =
        std::collections::HashMap::new();

    for annotation in &metadata.annotations {
        annotations_by_page
            .entry(annotation.page_index as usize)
            .or_default()
            .push(annotation);
    }

    // Add annotations to each page
    for (page_index, page_annotations) in annotations_by_page {
        if let Ok(mut page) = document.pages_mut().get(page_index as u16) {
            for annotation in page_annotations {
                if annotation.visible {
                    add_annotation_to_page(&mut page, annotation, &document)?;
                }
            }

            // Regenerate page content to commit changes
            page.regenerate_content()
                .map_err(|e| PdfExportError::GenerationError(e.to_string()))?;
        }
    }

    // Save the modified PDF
    document
        .save_to_file(output_path)
        .map_err(|e| PdfExportError::IoError(
            std::io::Error::other(e.to_string())
        ))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::annotation::{AnnotationGeometry, AnnotationMetadata, AnnotationStyle, Color, PageCoordinate};

    fn create_test_annotation(geometry: AnnotationGeometry) -> SerializableAnnotation {
        SerializableAnnotation {
            id: uuid::Uuid::new_v4(),
            page_index: 0,
            geometry,
            style: AnnotationStyle::new(),
            metadata: AnnotationMetadata::new(),
            visible: true,
            layer: 0,
        }
    }

    #[test]
    fn test_generate_appearance_stream_line() {
        let annotation = create_test_annotation(AnnotationGeometry::Line {
            start: PageCoordinate::new(10.0, 10.0),
            end: PageCoordinate::new(100.0, 100.0),
        });

        let stream = generate_appearance_stream(&annotation).unwrap();
        assert!(stream.contains("m")); // Move operation
        assert!(stream.contains("l")); // Line operation
        assert!(stream.contains("S")); // Stroke operation
    }

    #[test]
    fn test_generate_appearance_stream_circle() {
        let annotation = create_test_annotation(AnnotationGeometry::Circle {
            center: PageCoordinate::new(50.0, 50.0),
            radius: 25.0,
        });

        let stream = generate_appearance_stream(&annotation).unwrap();
        assert!(stream.contains("m")); // Move operation
        assert!(stream.contains("c")); // Curve operation
        assert!(stream.contains("s")); // Close and stroke
    }

    #[test]
    fn test_generate_appearance_stream_rectangle() {
        let annotation = create_test_annotation(AnnotationGeometry::Rectangle {
            top_left: PageCoordinate::new(10.0, 10.0),
            bottom_right: PageCoordinate::new(100.0, 100.0),
        });

        let stream = generate_appearance_stream(&annotation).unwrap();
        assert!(stream.contains("re")); // Rectangle operation
        assert!(stream.contains("S")); // Stroke operation
    }

    #[test]
    fn test_generate_appearance_stream_with_fill() {
        let mut style = AnnotationStyle::new();
        style.fill_color = Some(Color::YELLOW);

        let annotation = SerializableAnnotation {
            id: uuid::Uuid::new_v4(),
            page_index: 0,
            geometry: AnnotationGeometry::Rectangle {
                top_left: PageCoordinate::new(10.0, 10.0),
                bottom_right: PageCoordinate::new(100.0, 100.0),
            },
            style,
            metadata: AnnotationMetadata::new(),
            visible: true,
            layer: 0,
        };

        let stream = generate_appearance_stream(&annotation).unwrap();
        assert!(stream.contains("rg")); // Fill color
        assert!(stream.contains("B")); // Fill and stroke
    }

    #[test]
    fn test_to_pdf_color() {
        let color = Color::RED;
        let (r, g, b) = to_pdf_color(&color);
        assert_eq!(r, 1.0);
        assert_eq!(g, 0.0);
        assert_eq!(b, 0.0);
    }

    #[test]
    fn test_export_options_default() {
        let options = ExportOptions::default();
        assert!(options.include_annotations);
        assert!(options.generate_appearances);
        assert!(!options.flatten);
    }

    #[test]
    fn test_to_pdfium_color() {
        let color = Color::RED;
        let (r, g, b) = to_pdfium_color(&color);
        assert_eq!(r, 255);
        assert_eq!(g, 0);
        assert_eq!(b, 0);
    }
}
