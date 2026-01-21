//! Font information extraction from PDF documents
//!
//! This module provides utilities to extract font information from PDF pages,
//! enabling text editing operations to preserve the original font characteristics.
//!
//! Leverages pdfium-render's font APIs (FPDFFont_*() functions) to extract:
//! - Font names (embedded or standard)
//! - Font sizes
//! - Text bounding boxes with associated font info
//!
//! This information is critical for Task 116: "Preserve embedded fonts where possible"

use pdfium_render::prelude::*;
use std::collections::HashMap;

/// Information about a font used in a PDF page
#[derive(Debug, Clone, PartialEq)]
pub struct FontInfo {
    /// Font name (e.g., "Helvetica", "Times-Roman", "Arial-BoldMT")
    pub name: String,

    /// Whether this is one of the 14 standard PDF fonts
    pub is_standard: bool,

    /// Whether the font is embedded in the PDF
    pub is_embedded: bool,

    /// Font weight (if available)
    pub weight: Option<u16>,

    /// Whether the font is italic
    pub is_italic: bool,

    /// Whether the font is bold
    pub is_bold: bool,
}

impl FontInfo {
    /// Create a new font info
    pub fn new(name: String) -> Self {
        let is_bold = name.to_lowercase().contains("bold");
        let is_italic = name.to_lowercase().contains("italic") || name.to_lowercase().contains("oblique");

        Self {
            name,
            is_standard: false,
            is_embedded: false,
            weight: None,
            is_italic,
            is_bold,
        }
    }

    /// Check if this is one of the 14 standard PDF fonts
    pub fn check_is_standard(&mut self) {
        self.is_standard = matches!(
            self.name.as_str(),
            "Courier"
                | "Courier-Bold"
                | "Courier-Oblique"
                | "Courier-BoldOblique"
                | "Helvetica"
                | "Helvetica-Bold"
                | "Helvetica-Oblique"
                | "Helvetica-BoldOblique"
                | "Times-Roman"
                | "Times-Bold"
                | "Times-Italic"
                | "Times-BoldItalic"
                | "Symbol"
                | "ZapfDingbats"
        );
    }
}

/// Text span with associated font information
#[derive(Debug, Clone)]
pub struct TextSpanWithFont {
    /// Text content
    pub text: String,

    /// Bounding box in page coordinates (x, y, width, height)
    pub bbox: (f32, f32, f32, f32),

    /// Font size in points
    pub font_size: f32,

    /// Font information
    pub font: FontInfo,
}

/// Extract font information from all text objects on a PDF page
///
/// This function iterates through all text objects on a page and extracts
/// font information for each piece of text, including the font name, size,
/// and bounding box.
///
/// # Arguments
/// * `page` - Reference to the PDF page
///
/// # Returns
/// A vector of text spans with their associated font information
pub fn extract_fonts_from_page(page: &PdfPage) -> Result<Vec<TextSpanWithFont>, String> {
    let mut spans = Vec::new();

    // Get page objects
    let objects = page.objects();

    // Iterate through all objects on the page
    for object in objects.iter() {
        // Only process text objects
        if object.object_type() != PdfPageObjectType::Text {
            continue;
        }

        // Get the text object (PdfPageObject is an enum, need to match on Text variant)
        let text_object = match object {
            PdfPageObject::Text(ref text_obj) => text_obj,
            _ => continue, // Not a text object, skip
        };

        // Get the text content
        let text = text_object.text();

        // Skip empty text
        if text.trim().is_empty() {
            continue;
        }

        // Get the bounding box
        let bounds = object.bounds().map_err(|e| e.to_string())?;
        let bbox = (
            bounds.left().value,
            bounds.bottom().value,
            bounds.width().value,
            bounds.height().value,
        );

        // Extract font information
        let font = text_object.font();
        let font_name = font.name();

        // Get font size from the scaled height of the text object
        // PdfPageTextObject doesn't have font_size() method, use bounds height as estimate
        let font_size = bounds.height().value;

        // Create font info
        let mut font_info = FontInfo::new(font_name);
        font_info.check_is_standard();

        // Check if font is embedded (returns Result<bool>)
        font_info.is_embedded = font.is_embedded().unwrap_or(false);

        spans.push(TextSpanWithFont {
            text,
            bbox,
            font_size,
            font: font_info,
        });
    }

    Ok(spans)
}

/// Get a map of all unique fonts used on a page
///
/// This is useful for determining what fonts are available for text editing.
///
/// # Arguments
/// * `page` - Reference to the PDF page
///
/// # Returns
/// A map from font name to font information
pub fn get_page_fonts(page: &PdfPage) -> Result<HashMap<String, FontInfo>, String> {
    let spans = extract_fonts_from_page(page)?;

    let mut fonts = HashMap::new();
    for span in spans {
        fonts.insert(span.font.name.clone(), span.font);
    }

    Ok(fonts)
}

/// Find the most appropriate font for a text region
///
/// Given a bounding box on a page, find the font that is most commonly used
/// in that region. This is useful when creating text edits to preserve the
/// original font characteristics.
///
/// # Arguments
/// * `page` - Reference to the PDF page
/// * `bbox` - Bounding box to search within (x, y, width, height)
///
/// # Returns
/// The most common font info in that region, or None if no text is found
pub fn find_font_in_region(
    page: &PdfPage,
    bbox: (f32, f32, f32, f32),
) -> Result<Option<(FontInfo, f32)>, String> {
    let spans = extract_fonts_from_page(page)?;

    // Filter spans that overlap with the bounding box
    let overlapping: Vec<_> = spans
        .into_iter()
        .filter(|span| {
            let (sx, sy, sw, sh) = span.bbox;
            let (bx, by, bw, bh) = bbox;

            // Check for bounding box overlap
            !(sx + sw < bx || bx + bw < sx || sy + sh < by || by + bh < sy)
        })
        .collect();

    if overlapping.is_empty() {
        return Ok(None);
    }

    // Find the most common font
    let mut font_counts: HashMap<String, (FontInfo, f32, usize)> = HashMap::new();
    for span in overlapping {
        let entry = font_counts
            .entry(span.font.name.clone())
            .or_insert((span.font.clone(), span.font_size, 0));
        entry.2 += 1;
    }

    // Return the font with the highest count
    let (font, size, _) = font_counts
        .into_iter()
        .max_by_key(|(_, (_, _, count))| *count)
        .map(|(_, v)| v)
        .unwrap();

    Ok(Some((font, size)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_font_info_creation() {
        let font = FontInfo::new("Helvetica-Bold".to_string());
        assert_eq!(font.name, "Helvetica-Bold");
        assert!(font.is_bold);
        assert!(!font.is_italic);
    }

    #[test]
    fn test_font_info_italic_detection() {
        let font1 = FontInfo::new("Times-Italic".to_string());
        assert!(font1.is_italic);
        assert!(!font1.is_bold);

        let font2 = FontInfo::new("Helvetica-Oblique".to_string());
        assert!(font2.is_italic);
    }

    #[test]
    fn test_font_info_bold_detection() {
        let font = FontInfo::new("Arial-BoldMT".to_string());
        assert!(font.is_bold);
    }

    #[test]
    fn test_standard_font_check() {
        let mut font1 = FontInfo::new("Helvetica".to_string());
        font1.check_is_standard();
        assert!(font1.is_standard);

        let mut font2 = FontInfo::new("Times-Roman".to_string());
        font2.check_is_standard();
        assert!(font2.is_standard);

        let mut font3 = FontInfo::new("Arial".to_string());
        font3.check_is_standard();
        assert!(!font3.is_standard);

        let mut font4 = FontInfo::new("Custom-Font".to_string());
        font4.check_is_standard();
        assert!(!font4.is_standard);
    }

    #[test]
    fn test_all_14_standard_fonts() {
        let standard_fonts = vec![
            "Courier",
            "Courier-Bold",
            "Courier-Oblique",
            "Courier-BoldOblique",
            "Helvetica",
            "Helvetica-Bold",
            "Helvetica-Oblique",
            "Helvetica-BoldOblique",
            "Times-Roman",
            "Times-Bold",
            "Times-Italic",
            "Times-BoldItalic",
            "Symbol",
            "ZapfDingbats",
        ];

        for name in standard_fonts {
            let mut font = FontInfo::new(name.to_string());
            font.check_is_standard();
            assert!(
                font.is_standard,
                "Font {} should be recognized as standard",
                name
            );
        }
    }
}
