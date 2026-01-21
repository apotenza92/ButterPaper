//! Invisible text layer for OCR results
//!
//! Stores OCR-extracted text with precise page coordinate alignment for search and selection.
//! The text layer is invisible but enables text search and selection on scanned PDFs.

use crate::annotation::PageCoordinate;
use crate::ocr::OcrResult;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// A positioned text span in page coordinates
///
/// Represents a single piece of text with its bounding box in PDF page space.
/// Used for text search, selection, and copy operations.
#[derive(Debug, Clone, PartialEq)]
pub struct TextSpan {
    /// The text content
    pub text: String,

    /// Bounding box in page coordinates (x, y, width, height)
    /// Origin at bottom-left, coordinates in points (1/72 inch)
    pub bbox: TextBoundingBox,

    /// Confidence score from OCR (0.0 to 1.0)
    pub confidence: f32,

    /// Font size estimate in points
    pub font_size: f32,
}

impl TextSpan {
    /// Create a new text span
    pub fn new(text: String, bbox: TextBoundingBox, confidence: f32, font_size: f32) -> Self {
        Self {
            text,
            bbox,
            confidence,
            font_size,
        }
    }

    /// Check if this span contains a point in page coordinates
    pub fn contains_point(&self, point: &PageCoordinate) -> bool {
        self.bbox.contains_point(point)
    }

    /// Check if this span overlaps with a rectangle
    pub fn overlaps_rect(&self, rect: &TextBoundingBox) -> bool {
        self.bbox.overlaps(rect)
    }
}

/// Bounding box in page coordinates
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TextBoundingBox {
    /// X coordinate of bottom-left corner (points)
    pub x: f32,

    /// Y coordinate of bottom-left corner (points)
    pub y: f32,

    /// Width in points
    pub width: f32,

    /// Height in points
    pub height: f32,
}

impl TextBoundingBox {
    /// Create a new bounding box
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// Create from (x, y, width, height) tuple
    pub fn from_tuple(bbox: (f32, f32, f32, f32)) -> Self {
        Self::new(bbox.0, bbox.1, bbox.2, bbox.3)
    }

    /// Get the corners of this bounding box
    pub fn corners(&self) -> (PageCoordinate, PageCoordinate) {
        (
            PageCoordinate::new(self.x, self.y),
            PageCoordinate::new(self.x + self.width, self.y + self.height),
        )
    }

    /// Check if this bounding box contains a point
    pub fn contains_point(&self, point: &PageCoordinate) -> bool {
        point.x >= self.x
            && point.x <= self.x + self.width
            && point.y >= self.y
            && point.y <= self.y + self.height
    }

    /// Check if this bounding box overlaps with another
    pub fn overlaps(&self, other: &TextBoundingBox) -> bool {
        !(self.x + self.width < other.x
            || other.x + other.width < self.x
            || self.y + self.height < other.y
            || other.y + other.height < self.y)
    }

    /// Calculate the area of intersection with another bounding box
    pub fn intersection_area(&self, other: &TextBoundingBox) -> f32 {
        if !self.overlaps(other) {
            return 0.0;
        }

        let x_overlap = (self.x + self.width).min(other.x + other.width) - self.x.max(other.x);
        let y_overlap = (self.y + self.height).min(other.y + other.height) - self.y.max(other.y);

        x_overlap * y_overlap
    }
}

/// Text layer for a single page
///
/// Stores text spans extracted via OCR, aligned to page coordinates.
/// Enables text search and selection on pages without native selectable text.
#[derive(Debug, Clone)]
pub struct PageTextLayer {
    /// Page index this layer belongs to
    pub page_index: u16,

    /// Text spans in reading order
    pub spans: Vec<TextSpan>,

    /// Full text content (for fast search)
    pub text: String,

    /// Whether this layer came from OCR or native PDF text
    pub is_ocr: bool,

    /// Overall confidence (0.0 to 1.0), 1.0 for native text
    pub confidence: f32,
}

impl PageTextLayer {
    /// Create a new empty text layer
    pub fn new(page_index: u16) -> Self {
        Self {
            page_index,
            spans: Vec::new(),
            text: String::new(),
            is_ocr: false,
            confidence: 0.0,
        }
    }

    /// Create a text layer from OCR result
    pub fn from_ocr_result(result: &OcrResult) -> Self {
        let spans: Vec<TextSpan> = result
            .text_blocks
            .iter()
            .map(|block| {
                TextSpan::new(
                    block.text.clone(),
                    TextBoundingBox::from_tuple(block.bbox),
                    block.confidence,
                    block.font_size,
                )
            })
            .collect();

        Self {
            page_index: result.page_index,
            spans,
            text: result.text.clone(),
            is_ocr: !result.had_existing_text,
            confidence: result.confidence,
        }
    }

    /// Create a text layer from a list of text spans
    pub fn from_spans(page_index: u16, spans: Vec<TextSpan>) -> Self {
        let text = spans
            .iter()
            .map(|s| s.text.as_str())
            .collect::<Vec<_>>()
            .join(" ");

        let avg_confidence = if spans.is_empty() {
            0.0
        } else {
            spans.iter().map(|s| s.confidence).sum::<f32>() / spans.len() as f32
        };

        Self {
            page_index,
            spans,
            text,
            is_ocr: true,
            confidence: avg_confidence,
        }
    }

    /// Find all spans that overlap with a rectangle (for selection)
    pub fn find_spans_in_rect(&self, rect: &TextBoundingBox) -> Vec<&TextSpan> {
        self.spans
            .iter()
            .filter(|span| span.overlaps_rect(rect))
            .collect()
    }

    /// Find the span containing a specific point
    pub fn find_span_at_point(&self, point: &PageCoordinate) -> Option<&TextSpan> {
        self.spans.iter().find(|span| span.contains_point(point))
    }

    /// Search for text in this layer
    ///
    /// Returns the indices of matching spans and their character positions.
    /// If `case_sensitive` is false, the search is case-insensitive.
    pub fn search(&self, query: &str, case_sensitive: bool) -> Vec<SearchMatch> {
        let query_search = if case_sensitive {
            query.to_string()
        } else {
            query.to_lowercase()
        };
        let mut matches = Vec::new();
        let mut char_pos = 0;

        for (span_index, span) in self.spans.iter().enumerate() {
            let span_text_search = if case_sensitive {
                span.text.clone()
            } else {
                span.text.to_lowercase()
            };
            let mut search_pos = 0;

            while let Some(match_pos) = span_text_search[search_pos..].find(&query_search) {
                let absolute_pos = search_pos + match_pos;
                matches.push(SearchMatch {
                    span_index,
                    char_start: char_pos + absolute_pos,
                    char_end: char_pos + absolute_pos + query.len(),
                    text: span.text[absolute_pos..absolute_pos + query.len()].to_string(),
                    bbox: span.bbox,
                });
                search_pos = absolute_pos + 1;
            }

            char_pos += span.text.len() + 1; // +1 for space between spans
        }

        matches
    }

    /// Get the bounding boxes for a character range
    pub fn get_selection_boxes(&self, char_start: usize, char_end: usize) -> Vec<TextBoundingBox> {
        let mut boxes = Vec::new();
        let mut char_pos = 0;

        for span in &self.spans {
            let span_end = char_pos + span.text.len();

            if char_pos >= char_end {
                break;
            }

            if span_end > char_start {
                // This span is part of the selection
                boxes.push(span.bbox);
            }

            char_pos = span_end + 1; // +1 for space between spans
        }

        boxes
    }

    /// Check if this layer is reliable (high confidence)
    pub fn is_reliable(&self) -> bool {
        self.confidence >= 0.7 || !self.is_ocr
    }
}

/// A search match result
#[derive(Debug, Clone, PartialEq)]
pub struct SearchMatch {
    /// Index of the span containing this match
    pub span_index: usize,

    /// Character position of match start in full page text
    pub char_start: usize,

    /// Character position of match end in full page text
    pub char_end: usize,

    /// The matched text
    pub text: String,

    /// Bounding box of the span containing the match
    pub bbox: TextBoundingBox,
}

/// Text layer manager for the entire document
///
/// Thread-safe storage and retrieval of text layers for all pages.
/// Integrates with the OCR subsystem to populate layers progressively.
pub struct TextLayerManager {
    /// Text layers by page index
    layers: Arc<RwLock<HashMap<u16, PageTextLayer>>>,

    /// Total number of pages in the document
    total_pages: u16,
}

impl TextLayerManager {
    /// Create a new text layer manager
    pub fn new(total_pages: u16) -> Self {
        Self {
            layers: Arc::new(RwLock::new(HashMap::new())),
            total_pages,
        }
    }

    /// Add or update a text layer for a page
    pub fn set_layer(&self, layer: PageTextLayer) {
        let mut layers = self.layers.write().unwrap();
        layers.insert(layer.page_index, layer);
    }

    /// Add a text layer from an OCR result
    pub fn add_from_ocr(&self, result: &OcrResult) {
        let layer = PageTextLayer::from_ocr_result(result);
        self.set_layer(layer);
    }

    /// Get the text layer for a specific page
    pub fn get_layer(&self, page_index: u16) -> Option<PageTextLayer> {
        let layers = self.layers.read().unwrap();
        layers.get(&page_index).cloned()
    }

    /// Check if a page has a text layer
    pub fn has_layer(&self, page_index: u16) -> bool {
        let layers = self.layers.read().unwrap();
        layers.contains_key(&page_index)
    }

    /// Remove a text layer for a page
    pub fn remove_layer(&self, page_index: u16) {
        let mut layers = self.layers.write().unwrap();
        layers.remove(&page_index);
    }

    /// Clear all text layers
    pub fn clear(&self) {
        let mut layers = self.layers.write().unwrap();
        layers.clear();
    }

    /// Get the number of pages with text layers
    pub fn layer_count(&self) -> usize {
        let layers = self.layers.read().unwrap();
        layers.len()
    }

    /// Get all page indices that have text layers
    pub fn pages_with_layers(&self) -> Vec<u16> {
        let layers = self.layers.read().unwrap();
        let mut pages: Vec<u16> = layers.keys().copied().collect();
        pages.sort_unstable();
        pages
    }

    /// Search for text across all pages
    ///
    /// Returns matches grouped by page index.
    /// If `case_sensitive` is false, the search is case-insensitive.
    pub fn search_all(&self, query: &str, case_sensitive: bool) -> HashMap<u16, Vec<SearchMatch>> {
        let layers = self.layers.read().unwrap();
        let mut results = HashMap::new();

        for (page_index, layer) in layers.iter() {
            let matches = layer.search(query, case_sensitive);
            if !matches.is_empty() {
                results.insert(*page_index, matches);
            }
        }

        results
    }

    /// Get statistics about text layer coverage
    pub fn stats(&self) -> TextLayerStats {
        let layers = self.layers.read().unwrap();
        let total_layers = layers.len();
        let ocr_layers = layers.values().filter(|l| l.is_ocr).count();
        let reliable_layers = layers.values().filter(|l| l.is_reliable()).count();

        TextLayerStats {
            total_pages: self.total_pages,
            pages_with_layers: total_layers,
            ocr_layers,
            native_text_layers: total_layers - ocr_layers,
            reliable_layers,
            coverage: total_layers as f32 / self.total_pages as f32,
        }
    }

    /// Get the total number of pages
    pub fn total_pages(&self) -> u16 {
        self.total_pages
    }
}

/// Statistics about text layer coverage
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextLayerStats {
    /// Total number of pages in document
    pub total_pages: u16,

    /// Number of pages with text layers
    pub pages_with_layers: usize,

    /// Number of layers from OCR
    pub ocr_layers: usize,

    /// Number of layers from native PDF text
    pub native_text_layers: usize,

    /// Number of reliable layers (high confidence)
    pub reliable_layers: usize,

    /// Coverage ratio (0.0 to 1.0)
    pub coverage: f32,
}

impl TextLayerStats {
    /// Check if all pages have text layers
    pub fn is_complete(&self) -> bool {
        self.pages_with_layers == self.total_pages as usize
    }

    /// Get the percentage of pages with text layers
    pub fn coverage_percent(&self) -> f32 {
        self.coverage * 100.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ocr::TextBlock;

    #[test]
    fn test_text_bounding_box_creation() {
        let bbox = TextBoundingBox::new(10.0, 20.0, 100.0, 15.0);
        assert_eq!(bbox.x, 10.0);
        assert_eq!(bbox.y, 20.0);
        assert_eq!(bbox.width, 100.0);
        assert_eq!(bbox.height, 15.0);

        let bbox2 = TextBoundingBox::from_tuple((10.0, 20.0, 100.0, 15.0));
        assert_eq!(bbox, bbox2);
    }

    #[test]
    fn test_text_bounding_box_contains_point() {
        let bbox = TextBoundingBox::new(10.0, 20.0, 100.0, 15.0);

        assert!(bbox.contains_point(&PageCoordinate::new(50.0, 25.0)));
        assert!(bbox.contains_point(&PageCoordinate::new(10.0, 20.0))); // Corner
        assert!(bbox.contains_point(&PageCoordinate::new(110.0, 35.0))); // Corner
        assert!(!bbox.contains_point(&PageCoordinate::new(5.0, 25.0))); // Outside left
        assert!(!bbox.contains_point(&PageCoordinate::new(50.0, 15.0))); // Outside bottom
    }

    #[test]
    fn test_text_bounding_box_overlaps() {
        let bbox1 = TextBoundingBox::new(10.0, 20.0, 100.0, 15.0);
        let bbox2 = TextBoundingBox::new(50.0, 25.0, 100.0, 15.0);
        let bbox3 = TextBoundingBox::new(200.0, 20.0, 100.0, 15.0);

        assert!(bbox1.overlaps(&bbox2)); // Overlapping
        assert!(bbox2.overlaps(&bbox1)); // Symmetric
        assert!(!bbox1.overlaps(&bbox3)); // Separate
    }

    #[test]
    fn test_text_span_creation() {
        let bbox = TextBoundingBox::new(10.0, 20.0, 100.0, 15.0);
        let span = TextSpan::new("Hello".to_string(), bbox, 0.95, 12.0);

        assert_eq!(span.text, "Hello");
        assert_eq!(span.bbox, bbox);
        assert_eq!(span.confidence, 0.95);
        assert_eq!(span.font_size, 12.0);
    }

    #[test]
    fn test_page_text_layer_from_spans() {
        let spans = vec![
            TextSpan::new(
                "Hello".to_string(),
                TextBoundingBox::new(10.0, 20.0, 50.0, 15.0),
                0.95,
                12.0,
            ),
            TextSpan::new(
                "World".to_string(),
                TextBoundingBox::new(65.0, 20.0, 50.0, 15.0),
                0.90,
                12.0,
            ),
        ];

        let layer = PageTextLayer::from_spans(0, spans);
        assert_eq!(layer.page_index, 0);
        assert_eq!(layer.spans.len(), 2);
        assert_eq!(layer.text, "Hello World");
        assert!(layer.is_ocr);
        assert!((layer.confidence - 0.925).abs() < 0.001); // Average of 0.95 and 0.90
    }

    #[test]
    fn test_page_text_layer_find_span_at_point() {
        let spans = vec![
            TextSpan::new(
                "Hello".to_string(),
                TextBoundingBox::new(10.0, 20.0, 50.0, 15.0),
                0.95,
                12.0,
            ),
            TextSpan::new(
                "World".to_string(),
                TextBoundingBox::new(65.0, 20.0, 50.0, 15.0),
                0.90,
                12.0,
            ),
        ];

        let layer = PageTextLayer::from_spans(0, spans);

        let span = layer.find_span_at_point(&PageCoordinate::new(30.0, 25.0));
        assert!(span.is_some());
        assert_eq!(span.unwrap().text, "Hello");

        let span = layer.find_span_at_point(&PageCoordinate::new(80.0, 25.0));
        assert!(span.is_some());
        assert_eq!(span.unwrap().text, "World");

        let span = layer.find_span_at_point(&PageCoordinate::new(200.0, 25.0));
        assert!(span.is_none());
    }

    #[test]
    fn test_page_text_layer_search() {
        let spans = vec![
            TextSpan::new(
                "Hello".to_string(),
                TextBoundingBox::new(10.0, 20.0, 50.0, 15.0),
                0.95,
                12.0,
            ),
            TextSpan::new(
                "World".to_string(),
                TextBoundingBox::new(65.0, 20.0, 50.0, 15.0),
                0.90,
                12.0,
            ),
            TextSpan::new(
                "Hello".to_string(),
                TextBoundingBox::new(120.0, 20.0, 50.0, 15.0),
                0.92,
                12.0,
            ),
        ];

        let layer = PageTextLayer::from_spans(0, spans);

        // Case-insensitive search (default)
        let matches = layer.search("hello", false);
        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0].text, "Hello");
        assert_eq!(matches[1].text, "Hello");

        let matches = layer.search("world", false);
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].text, "World");

        let matches = layer.search("notfound", false);
        assert_eq!(matches.len(), 0);
    }

    #[test]
    fn test_page_text_layer_search_case_sensitive() {
        let spans = vec![
            TextSpan::new(
                "Hello".to_string(),
                TextBoundingBox::new(10.0, 20.0, 50.0, 15.0),
                0.95,
                12.0,
            ),
            TextSpan::new(
                "hello".to_string(),
                TextBoundingBox::new(65.0, 20.0, 50.0, 15.0),
                0.90,
                12.0,
            ),
            TextSpan::new(
                "HELLO".to_string(),
                TextBoundingBox::new(120.0, 20.0, 50.0, 15.0),
                0.92,
                12.0,
            ),
        ];

        let layer = PageTextLayer::from_spans(0, spans);

        // Case-sensitive search
        let matches = layer.search("Hello", true);
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].text, "Hello");

        let matches = layer.search("hello", true);
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].text, "hello");

        let matches = layer.search("HELLO", true);
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].text, "HELLO");

        // Case-insensitive search should find all three
        let matches = layer.search("hello", false);
        assert_eq!(matches.len(), 3);
    }

    #[test]
    fn test_text_layer_manager_operations() {
        let manager = TextLayerManager::new(10);

        assert_eq!(manager.total_pages(), 10);
        assert_eq!(manager.layer_count(), 0);
        assert!(!manager.has_layer(0));

        let layer = PageTextLayer::from_spans(
            0,
            vec![TextSpan::new(
                "Test".to_string(),
                TextBoundingBox::new(0.0, 0.0, 50.0, 15.0),
                0.9,
                12.0,
            )],
        );

        manager.set_layer(layer.clone());
        assert_eq!(manager.layer_count(), 1);
        assert!(manager.has_layer(0));

        let retrieved = manager.get_layer(0);
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().page_index, 0);

        manager.remove_layer(0);
        assert_eq!(manager.layer_count(), 0);
        assert!(!manager.has_layer(0));
    }

    #[test]
    fn test_text_layer_manager_search() {
        let manager = TextLayerManager::new(3);

        let layer1 = PageTextLayer::from_spans(
            0,
            vec![TextSpan::new(
                "Hello World".to_string(),
                TextBoundingBox::new(0.0, 0.0, 100.0, 15.0),
                0.9,
                12.0,
            )],
        );

        let layer2 = PageTextLayer::from_spans(
            1,
            vec![TextSpan::new(
                "World Peace".to_string(),
                TextBoundingBox::new(0.0, 0.0, 100.0, 15.0),
                0.85,
                12.0,
            )],
        );

        manager.set_layer(layer1);
        manager.set_layer(layer2);

        // Case-insensitive search
        let results = manager.search_all("world", false);
        assert_eq!(results.len(), 2);
        assert!(results.contains_key(&0));
        assert!(results.contains_key(&1));
    }

    #[test]
    fn test_text_layer_manager_search_case_sensitive() {
        let manager = TextLayerManager::new(3);

        let layer1 = PageTextLayer::from_spans(
            0,
            vec![TextSpan::new(
                "Hello World".to_string(),
                TextBoundingBox::new(0.0, 0.0, 100.0, 15.0),
                0.9,
                12.0,
            )],
        );

        let layer2 = PageTextLayer::from_spans(
            1,
            vec![TextSpan::new(
                "world peace".to_string(),
                TextBoundingBox::new(0.0, 0.0, 100.0, 15.0),
                0.85,
                12.0,
            )],
        );

        manager.set_layer(layer1);
        manager.set_layer(layer2);

        // Case-sensitive search for "World" should only find page 0
        let results = manager.search_all("World", true);
        assert_eq!(results.len(), 1);
        assert!(results.contains_key(&0));
        assert!(!results.contains_key(&1));

        // Case-sensitive search for "world" should only find page 1
        let results = manager.search_all("world", true);
        assert_eq!(results.len(), 1);
        assert!(!results.contains_key(&0));
        assert!(results.contains_key(&1));

        // Case-insensitive search should find both
        let results = manager.search_all("world", false);
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_text_layer_stats() {
        let manager = TextLayerManager::new(10);

        let mut stats = manager.stats();
        assert_eq!(stats.total_pages, 10);
        assert_eq!(stats.pages_with_layers, 0);
        assert!(!stats.is_complete());
        assert_eq!(stats.coverage_percent(), 0.0);

        for i in 0..10 {
            let layer = PageTextLayer::from_spans(
                i,
                vec![TextSpan::new(
                    "Test".to_string(),
                    TextBoundingBox::new(0.0, 0.0, 50.0, 15.0),
                    0.9,
                    12.0,
                )],
            );
            manager.set_layer(layer);
        }

        stats = manager.stats();
        assert_eq!(stats.pages_with_layers, 10);
        assert!(stats.is_complete());
        assert_eq!(stats.coverage_percent(), 100.0);
        assert_eq!(stats.ocr_layers, 10);
    }

    #[test]
    fn test_page_text_layer_from_ocr_result() {
        let text_blocks = vec![
            TextBlock::new("Hello".to_string(), (10.0, 20.0, 50.0, 15.0), 0.95, 12.0),
            TextBlock::new("World".to_string(), (65.0, 20.0, 50.0, 15.0), 0.90, 12.0),
        ];

        let ocr_result = OcrResult::new(0, "Hello World".to_string(), text_blocks, 0.92);

        let layer = PageTextLayer::from_ocr_result(&ocr_result);
        assert_eq!(layer.page_index, 0);
        assert_eq!(layer.spans.len(), 2);
        assert_eq!(layer.text, "Hello World");
        assert!(layer.is_ocr);
        assert_eq!(layer.confidence, 0.92);
    }

    #[test]
    fn test_text_layer_manager_add_from_ocr() {
        let manager = TextLayerManager::new(5);

        let text_blocks = vec![TextBlock::new(
            "Test".to_string(),
            (0.0, 0.0, 50.0, 15.0),
            0.9,
            12.0,
        )];

        let ocr_result = OcrResult::new(0, "Test".to_string(), text_blocks, 0.9);

        manager.add_from_ocr(&ocr_result);
        assert!(manager.has_layer(0));

        let layer = manager.get_layer(0).unwrap();
        assert_eq!(layer.page_index, 0);
        assert_eq!(layer.text, "Test");
    }

    /// Test that verifies the lazy loading pattern works correctly:
    /// - has_layer() returns false for pages not yet extracted
    /// - set_layer() only adds layers once
    /// - Layers can be added incrementally (simulating on-demand extraction)
    #[test]
    fn test_lazy_loading_pattern() {
        let manager = TextLayerManager::new(100); // Simulate 100-page PDF

        // Initially no pages have text layers
        assert_eq!(manager.layer_count(), 0);
        for i in 0..100 {
            assert!(!manager.has_layer(i), "Page {} should not have layer yet", i);
        }

        // Simulate lazy loading: extract only page 0 on initial load
        let layer0 = PageTextLayer::from_spans(
            0,
            vec![TextSpan::new(
                "First page text".to_string(),
                TextBoundingBox::new(0.0, 0.0, 100.0, 15.0),
                1.0,
                12.0,
            )],
        );
        manager.set_layer(layer0);

        assert_eq!(manager.layer_count(), 1);
        assert!(manager.has_layer(0));
        assert!(!manager.has_layer(1));

        // Simulate user navigates to page 5 - should check has_layer before extracting
        assert!(!manager.has_layer(5));
        let layer5 = PageTextLayer::from_spans(
            5,
            vec![TextSpan::new(
                "Page 5 text".to_string(),
                TextBoundingBox::new(0.0, 0.0, 80.0, 15.0),
                1.0,
                12.0,
            )],
        );
        manager.set_layer(layer5);

        assert_eq!(manager.layer_count(), 2);
        assert!(manager.has_layer(0));
        assert!(manager.has_layer(5));
        assert!(!manager.has_layer(1)); // Still not extracted

        // Verify search works with partial extraction (only finds text in extracted pages)
        let results = manager.search_all("text", false);
        assert_eq!(results.len(), 2);
        assert!(results.contains_key(&0));
        assert!(results.contains_key(&5));

        // Simulate bulk extraction when user initiates search
        for i in 0..10 {
            if !manager.has_layer(i) {
                let layer = PageTextLayer::from_spans(
                    i,
                    vec![TextSpan::new(
                        format!("Page {} content", i),
                        TextBoundingBox::new(0.0, 0.0, 100.0, 15.0),
                        1.0,
                        12.0,
                    )],
                );
                manager.set_layer(layer);
            }
        }

        assert_eq!(manager.layer_count(), 10);
        // Search should now find all pages
        let results = manager.search_all("content", false);
        assert_eq!(results.len(), 8); // Pages 0 and 5 have "text", not "content"
    }
}
