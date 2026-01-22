use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use uuid::Uuid;

use crate::text_layer::TextBoundingBox;

/// Unique identifier for a text edit
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TextEditId(Uuid);

impl TextEditId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for TextEditId {
    fn default() -> Self {
        Self::new()
    }
}

/// Font information for text edits
///
/// Stores comprehensive font information extracted from the PDF or specified by the user.
/// This enables text edits to preserve the original font characteristics where possible.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TextEditFont {
    /// Font name (e.g., "Helvetica", "Times-Roman", "Arial-BoldMT")
    pub name: String,

    /// Whether this is one of the 14 standard PDF fonts
    pub is_standard: bool,

    /// Whether the font is embedded in the PDF
    pub is_embedded: bool,

    /// Whether the font is bold
    pub is_bold: bool,

    /// Whether the font is italic
    pub is_italic: bool,

    /// Font weight (if available)
    pub weight: Option<u16>,
}

impl TextEditFont {
    /// Create a new font with standard characteristics
    pub fn new(name: String) -> Self {
        let is_bold = name.to_lowercase().contains("bold");
        let is_italic =
            name.to_lowercase().contains("italic") || name.to_lowercase().contains("oblique");

        Self {
            name,
            is_standard: false,
            is_embedded: false,
            is_bold,
            is_italic,
            weight: None,
        }
    }

    /// Create a default font (Helvetica)
    pub fn default_font() -> Self {
        let mut font = Self::new("Helvetica".to_string());
        font.is_standard = true;
        font
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

/// Represents a single text edit operation on a PDF page
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextEdit {
    /// Unique identifier for this edit
    pub id: TextEditId,

    /// Page index this edit belongs to
    pub page_index: u16,

    /// Bounding box of the edited text in page coordinates (points)
    pub bbox: TextBoundingBox,

    /// Original text before editing
    pub original_text: String,

    /// Edited text
    pub edited_text: String,

    /// Font size in points (preserved from original or user-specified)
    pub font_size: f32,

    /// Font information (preserved from original text where possible)
    pub font: TextEditFont,

    /// Font family name (deprecated, use font.name instead)
    /// Kept for backwards compatibility with existing serialized data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub font_family: Option<String>,

    /// RGB color for the edited text (0.0 to 1.0)
    pub color: [f32; 3],

    /// Whether this edit is visible
    pub visible: bool,

    /// Timestamp when edit was created
    pub created_at: i64,

    /// Timestamp when edit was last modified
    pub modified_at: i64,

    /// Optional author/user identifier
    pub author: Option<String>,

    /// Optional notes or metadata about this edit
    pub notes: Option<String>,
}

impl TextEdit {
    /// Create a new text edit with default font
    pub fn new(
        page_index: u16,
        bbox: TextBoundingBox,
        original_text: String,
        edited_text: String,
        font_size: f32,
    ) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        Self {
            id: TextEditId::new(),
            page_index,
            bbox,
            original_text,
            edited_text,
            font_size,
            font: TextEditFont::default_font(),
            font_family: None,
            color: [0.0, 0.0, 0.0], // Default to black text
            visible: true,
            created_at: now,
            modified_at: now,
            author: None,
            notes: None,
        }
    }

    /// Create a new text edit with specified font
    pub fn new_with_font(
        page_index: u16,
        bbox: TextBoundingBox,
        original_text: String,
        edited_text: String,
        font_size: f32,
        font: TextEditFont,
    ) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let font_family_name = font.name.clone();

        Self {
            id: TextEditId::new(),
            page_index,
            bbox,
            original_text,
            edited_text,
            font_size,
            font,
            font_family: Some(font_family_name),
            color: [0.0, 0.0, 0.0], // Default to black text
            visible: true,
            created_at: now,
            modified_at: now,
            author: None,
            notes: None,
        }
    }

    /// Update the font for this edit
    pub fn update_font(&mut self, font: TextEditFont) {
        self.font = font;
        self.font_family = Some(self.font.name.clone());
        self.modified_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
    }

    /// Get whether the font is available for use
    /// Standard or embedded fonts can be used; others require fallback
    pub fn is_font_available(&self) -> bool {
        self.font.is_standard || self.font.is_embedded
    }

    /// Get a suitable fallback font if the current font is not available
    pub fn get_fallback_font(&self) -> TextEditFont {
        // If the font is available, return it as-is
        if self.is_font_available() {
            return self.font.clone();
        }

        // Otherwise, choose an appropriate standard font based on characteristics
        let font_name = if self.font.is_bold && self.font.is_italic {
            "Helvetica-BoldOblique"
        } else if self.font.is_bold {
            "Helvetica-Bold"
        } else if self.font.is_italic {
            "Helvetica-Oblique"
        } else {
            "Helvetica"
        };

        let mut fallback = TextEditFont::new(font_name.to_string());
        fallback.check_is_standard();
        fallback
    }

    /// Update the edited text
    pub fn update_text(&mut self, new_text: String) {
        self.edited_text = new_text;
        self.modified_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
    }

    /// Check if this edit contains actual changes
    pub fn has_changes(&self) -> bool {
        self.original_text != self.edited_text
    }

    /// Check if the edit overlaps with a given bounding box
    pub fn overlaps(&self, other_bbox: &TextBoundingBox) -> bool {
        self.bbox.intersects(other_bbox)
    }
}

/// Manages all text edits for a single page
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageTextEdits {
    /// Page index
    pub page_index: u16,

    /// All text edits on this page
    pub edits: Vec<TextEdit>,
}

impl PageTextEdits {
    pub fn new(page_index: u16) -> Self {
        Self {
            page_index,
            edits: Vec::new(),
        }
    }

    /// Add a new text edit
    pub fn add_edit(&mut self, edit: TextEdit) {
        self.edits.push(edit);
    }

    /// Remove an edit by ID
    pub fn remove_edit(&mut self, edit_id: TextEditId) -> Option<TextEdit> {
        if let Some(index) = self.edits.iter().position(|e| e.id == edit_id) {
            Some(self.edits.remove(index))
        } else {
            None
        }
    }

    /// Get an edit by ID
    pub fn get_edit(&self, edit_id: TextEditId) -> Option<&TextEdit> {
        self.edits.iter().find(|e| e.id == edit_id)
    }

    /// Get a mutable reference to an edit by ID
    pub fn get_edit_mut(&mut self, edit_id: TextEditId) -> Option<&mut TextEdit> {
        self.edits.iter_mut().find(|e| e.id == edit_id)
    }

    /// Get all edits that overlap with a bounding box
    pub fn get_edits_in_bbox(&self, bbox: &TextBoundingBox) -> Vec<&TextEdit> {
        self.edits
            .iter()
            .filter(|e| e.visible && e.overlaps(bbox))
            .collect()
    }

    /// Get all visible edits
    pub fn visible_edits(&self) -> Vec<&TextEdit> {
        self.edits.iter().filter(|e| e.visible).collect()
    }

    /// Get the number of edits with actual changes
    pub fn changed_edit_count(&self) -> usize {
        self.edits.iter().filter(|e| e.has_changes()).count()
    }
}

/// Thread-safe manager for all text edits in a document
#[derive(Debug, Clone)]
pub struct TextEditManager {
    /// Map of page index to page text edits
    edits: Arc<RwLock<HashMap<u16, PageTextEdits>>>,
}

impl TextEditManager {
    /// Create a new empty text edit manager
    pub fn new() -> Self {
        Self {
            edits: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Load text edits from serialized data
    pub fn from_pages(pages: Vec<PageTextEdits>) -> Self {
        let mut edits = HashMap::new();
        for page_edits in pages {
            edits.insert(page_edits.page_index, page_edits);
        }

        Self {
            edits: Arc::new(RwLock::new(edits)),
        }
    }

    /// Add a text edit to a specific page
    pub fn add_edit(&self, edit: TextEdit) -> Result<(), String> {
        let mut edits = self.edits.write().map_err(|e| e.to_string())?;

        let page_edits = edits
            .entry(edit.page_index)
            .or_insert_with(|| PageTextEdits::new(edit.page_index));

        page_edits.add_edit(edit);
        Ok(())
    }

    /// Remove a text edit by ID
    pub fn remove_edit(
        &self,
        page_index: u16,
        edit_id: TextEditId,
    ) -> Result<Option<TextEdit>, String> {
        let mut edits = self.edits.write().map_err(|e| e.to_string())?;

        if let Some(page_edits) = edits.get_mut(&page_index) {
            Ok(page_edits.remove_edit(edit_id))
        } else {
            Ok(None)
        }
    }

    /// Update an existing text edit
    pub fn update_edit_text(
        &self,
        page_index: u16,
        edit_id: TextEditId,
        new_text: String,
    ) -> Result<bool, String> {
        let mut edits = self.edits.write().map_err(|e| e.to_string())?;

        if let Some(page_edits) = edits.get_mut(&page_index) {
            if let Some(edit) = page_edits.get_edit_mut(edit_id) {
                edit.update_text(new_text);
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Get all edits for a specific page
    pub fn get_page_edits(&self, page_index: u16) -> Result<Vec<TextEdit>, String> {
        let edits = self.edits.read().map_err(|e| e.to_string())?;

        if let Some(page_edits) = edits.get(&page_index) {
            Ok(page_edits.edits.clone())
        } else {
            Ok(Vec::new())
        }
    }

    /// Get all visible edits for a specific page
    pub fn get_visible_page_edits(&self, page_index: u16) -> Result<Vec<TextEdit>, String> {
        let edits = self.edits.read().map_err(|e| e.to_string())?;

        if let Some(page_edits) = edits.get(&page_index) {
            Ok(page_edits.visible_edits().into_iter().cloned().collect())
        } else {
            Ok(Vec::new())
        }
    }

    /// Get edits in a specific bounding box on a page
    pub fn get_edits_in_bbox(
        &self,
        page_index: u16,
        bbox: &TextBoundingBox,
    ) -> Result<Vec<TextEdit>, String> {
        let edits = self.edits.read().map_err(|e| e.to_string())?;

        if let Some(page_edits) = edits.get(&page_index) {
            Ok(page_edits
                .get_edits_in_bbox(bbox)
                .into_iter()
                .cloned()
                .collect())
        } else {
            Ok(Vec::new())
        }
    }

    /// Set visibility of an edit
    pub fn set_edit_visibility(
        &self,
        page_index: u16,
        edit_id: TextEditId,
        visible: bool,
    ) -> Result<bool, String> {
        let mut edits = self.edits.write().map_err(|e| e.to_string())?;

        if let Some(page_edits) = edits.get_mut(&page_index) {
            if let Some(edit) = page_edits.get_edit_mut(edit_id) {
                edit.visible = visible;
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Get total number of edits across all pages
    pub fn total_edit_count(&self) -> Result<usize, String> {
        let edits = self.edits.read().map_err(|e| e.to_string())?;
        Ok(edits.values().map(|p| p.edits.len()).sum())
    }

    /// Get number of edits with actual changes across all pages
    pub fn changed_edit_count(&self) -> Result<usize, String> {
        let edits = self.edits.read().map_err(|e| e.to_string())?;
        Ok(edits.values().map(|p| p.changed_edit_count()).sum())
    }

    /// Clear all edits from a specific page
    pub fn clear_page_edits(&self, page_index: u16) -> Result<(), String> {
        let mut edits = self.edits.write().map_err(|e| e.to_string())?;
        edits.remove(&page_index);
        Ok(())
    }

    /// Clear all edits from the document
    pub fn clear_all_edits(&self) -> Result<(), String> {
        let mut edits = self.edits.write().map_err(|e| e.to_string())?;
        edits.clear();
        Ok(())
    }

    /// Export all edits for serialization
    pub fn export_all(&self) -> Result<Vec<PageTextEdits>, String> {
        let edits = self.edits.read().map_err(|e| e.to_string())?;
        let mut pages: Vec<_> = edits.values().cloned().collect();
        pages.sort_by_key(|p| p.page_index);
        Ok(pages)
    }

    /// Get list of all page indices that have edits
    pub fn pages_with_edits(&self) -> Result<Vec<u16>, String> {
        let edits = self.edits.read().map_err(|e| e.to_string())?;
        let mut pages: Vec<_> = edits.keys().copied().collect();
        pages.sort_unstable();
        Ok(pages)
    }
}

impl Default for TextEditManager {
    fn default() -> Self {
        Self::new()
    }
}

impl TextBoundingBox {
    /// Check if this bounding box intersects with another
    pub fn intersects(&self, other: &TextBoundingBox) -> bool {
        let self_x2 = self.x + self.width;
        let self_y2 = self.y + self.height;
        let other_x2 = other.x + other.width;
        let other_y2 = other.y + other.height;

        !(self_x2 < other.x || other_x2 < self.x || self_y2 < other.y || other_y2 < self.y)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_edit_creation() {
        let bbox = TextBoundingBox {
            x: 100.0,
            y: 200.0,
            width: 150.0,
            height: 20.0,
        };

        let edit = TextEdit::new(0, bbox, "Original".to_string(), "Edited".to_string(), 12.0);

        assert_eq!(edit.page_index, 0);
        assert_eq!(edit.original_text, "Original");
        assert_eq!(edit.edited_text, "Edited");
        assert!(edit.has_changes());
        assert_eq!(edit.font.name, "Helvetica");
        assert!(edit.font.is_standard);
    }

    #[test]
    fn test_text_edit_with_custom_font() {
        let bbox = TextBoundingBox {
            x: 100.0,
            y: 200.0,
            width: 150.0,
            height: 20.0,
        };

        let mut font = TextEditFont::new("Arial-BoldMT".to_string());
        font.is_embedded = true;

        let edit = TextEdit::new_with_font(
            0,
            bbox,
            "Original".to_string(),
            "Edited".to_string(),
            12.0,
            font,
        );

        assert_eq!(edit.font.name, "Arial-BoldMT");
        assert!(edit.font.is_bold);
        assert!(edit.font.is_embedded);
        assert!(edit.is_font_available());
    }

    #[test]
    fn test_text_edit_no_changes() {
        let bbox = TextBoundingBox {
            x: 100.0,
            y: 200.0,
            width: 150.0,
            height: 20.0,
        };

        let edit = TextEdit::new(0, bbox, "Same".to_string(), "Same".to_string(), 12.0);

        assert!(!edit.has_changes());
    }

    #[test]
    fn test_font_availability() {
        let bbox = TextBoundingBox {
            x: 100.0,
            y: 200.0,
            width: 150.0,
            height: 20.0,
        };

        // Standard font is available
        let mut standard_font = TextEditFont::new("Helvetica".to_string());
        standard_font.check_is_standard();
        let edit1 = TextEdit::new_with_font(
            0,
            bbox,
            "Test".to_string(),
            "Test".to_string(),
            12.0,
            standard_font,
        );
        assert!(edit1.is_font_available());

        // Embedded font is available
        let mut embedded_font = TextEditFont::new("CustomFont".to_string());
        embedded_font.is_embedded = true;
        let edit2 = TextEdit::new_with_font(
            0,
            bbox,
            "Test".to_string(),
            "Test".to_string(),
            12.0,
            embedded_font,
        );
        assert!(edit2.is_font_available());

        // Non-embedded, non-standard font is not available
        let custom_font = TextEditFont::new("UnknownFont".to_string());
        let edit3 = TextEdit::new_with_font(
            0,
            bbox,
            "Test".to_string(),
            "Test".to_string(),
            12.0,
            custom_font,
        );
        assert!(!edit3.is_font_available());
    }

    #[test]
    fn test_fallback_font() {
        let bbox = TextBoundingBox {
            x: 100.0,
            y: 200.0,
            width: 150.0,
            height: 20.0,
        };

        // Bold italic custom font should fallback to Helvetica-BoldOblique
        let mut font1 = TextEditFont::new("CustomFont-BoldItalic".to_string());
        font1.is_bold = true;
        font1.is_italic = true;
        let edit1 =
            TextEdit::new_with_font(0, bbox, "Test".to_string(), "Test".to_string(), 12.0, font1);
        let fallback1 = edit1.get_fallback_font();
        assert_eq!(fallback1.name, "Helvetica-BoldOblique");
        assert!(fallback1.is_standard);

        // Bold custom font should fallback to Helvetica-Bold
        let mut font2 = TextEditFont::new("CustomFont-Bold".to_string());
        font2.is_italic = false;
        let edit2 =
            TextEdit::new_with_font(0, bbox, "Test".to_string(), "Test".to_string(), 12.0, font2);
        let fallback2 = edit2.get_fallback_font();
        assert_eq!(fallback2.name, "Helvetica-Bold");

        // Italic custom font should fallback to Helvetica-Oblique
        let mut font3 = TextEditFont::new("CustomFont-Italic".to_string());
        font3.is_bold = false;
        let edit3 =
            TextEdit::new_with_font(0, bbox, "Test".to_string(), "Test".to_string(), 12.0, font3);
        let fallback3 = edit3.get_fallback_font();
        assert_eq!(fallback3.name, "Helvetica-Oblique");

        // Regular custom font should fallback to Helvetica
        let font4 = TextEditFont::new("CustomFont".to_string());
        let edit4 =
            TextEdit::new_with_font(0, bbox, "Test".to_string(), "Test".to_string(), 12.0, font4);
        let fallback4 = edit4.get_fallback_font();
        assert_eq!(fallback4.name, "Helvetica");
    }

    #[test]
    fn test_font_update() {
        let bbox = TextBoundingBox {
            x: 100.0,
            y: 200.0,
            width: 150.0,
            height: 20.0,
        };

        let mut edit = TextEdit::new(0, bbox, "Test".to_string(), "Test".to_string(), 12.0);

        assert_eq!(edit.font.name, "Helvetica");

        let mut new_font = TextEditFont::new("Times-Roman".to_string());
        new_font.check_is_standard();
        edit.update_font(new_font);

        assert_eq!(edit.font.name, "Times-Roman");
        assert!(edit.font.is_standard);
    }

    #[test]
    fn test_bbox_intersection() {
        let bbox1 = TextBoundingBox {
            x: 0.0,
            y: 0.0,
            width: 100.0,
            height: 100.0,
        };

        let bbox2 = TextBoundingBox {
            x: 50.0,
            y: 50.0,
            width: 100.0,
            height: 100.0,
        };

        let bbox3 = TextBoundingBox {
            x: 200.0,
            y: 200.0,
            width: 100.0,
            height: 100.0,
        };

        assert!(bbox1.intersects(&bbox2));
        assert!(bbox2.intersects(&bbox1));
        assert!(!bbox1.intersects(&bbox3));
        assert!(!bbox3.intersects(&bbox1));
    }

    #[test]
    fn test_text_edit_manager() {
        let manager = TextEditManager::new();

        let bbox = TextBoundingBox {
            x: 100.0,
            y: 200.0,
            width: 150.0,
            height: 20.0,
        };

        let edit = TextEdit::new(0, bbox, "Original".to_string(), "Edited".to_string(), 12.0);

        let edit_id = edit.id;

        manager.add_edit(edit).unwrap();

        let edits = manager.get_page_edits(0).unwrap();
        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].id, edit_id);

        manager.remove_edit(0, edit_id).unwrap();
        let edits = manager.get_page_edits(0).unwrap();
        assert_eq!(edits.len(), 0);
    }
}
