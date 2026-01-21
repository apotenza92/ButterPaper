//! Bridge between render crate's FontInfo and core crate's TextEditFont
//!
//! This module provides conversion functions to preserve font information
//! when creating text edits from PDF content.

use crate::text_edit::TextEditFont;
use pdf_editor_render::FontInfo;

/// Convert a FontInfo from the render crate to a TextEditFont
///
/// This allows text edits to preserve the original font characteristics
/// extracted from the PDF page.
pub fn font_info_to_text_edit_font(font_info: &FontInfo) -> TextEditFont {
    TextEditFont {
        name: font_info.name.clone(),
        is_standard: font_info.is_standard,
        is_embedded: font_info.is_embedded,
        is_bold: font_info.is_bold,
        is_italic: font_info.is_italic,
        weight: font_info.weight,
    }
}

/// Extract font information for a text region from a PDF page
///
/// Given a bounding box, this function finds the most appropriate font
/// to use for a text edit in that region.
///
/// # Arguments
/// * `page` - Reference to the PDF page
/// * `bbox` - Bounding box in page coordinates (x, y, width, height)
///
/// # Returns
/// A tuple of (TextEditFont, font_size) if found, or None
pub fn extract_font_for_region(
    page: &pdfium_render::prelude::PdfPage,
    bbox: (f32, f32, f32, f32),
) -> Result<Option<(TextEditFont, f32)>, String> {
    use pdf_editor_render::find_font_in_region;

    let result = find_font_in_region(page, bbox)?;

    Ok(result.map(|(font_info, size)| {
        (font_info_to_text_edit_font(&font_info), size)
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_font_info_conversion() {
        let font_info = FontInfo {
            name: "Helvetica-Bold".to_string(),
            is_standard: true,
            is_embedded: false,
            weight: Some(700),
            is_italic: false,
            is_bold: true,
        };

        let text_edit_font = font_info_to_text_edit_font(&font_info);

        assert_eq!(text_edit_font.name, "Helvetica-Bold");
        assert!(text_edit_font.is_standard);
        assert!(!text_edit_font.is_embedded);
        assert!(text_edit_font.is_bold);
        assert!(!text_edit_font.is_italic);
        assert_eq!(text_edit_font.weight, Some(700));
    }

    #[test]
    fn test_embedded_font_conversion() {
        let font_info = FontInfo {
            name: "CustomFont-Regular".to_string(),
            is_standard: false,
            is_embedded: true,
            weight: None,
            is_italic: false,
            is_bold: false,
        };

        let text_edit_font = font_info_to_text_edit_font(&font_info);

        assert_eq!(text_edit_font.name, "CustomFont-Regular");
        assert!(!text_edit_font.is_standard);
        assert!(text_edit_font.is_embedded);
        assert!(!text_edit_font.is_bold);
        assert!(!text_edit_font.is_italic);
    }
}
