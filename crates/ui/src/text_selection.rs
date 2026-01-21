//! Text selection and search for OCR-enabled documents
//!
//! This module provides text selection and search capabilities for pages with OCR text layers.
//! It integrates with the TextLayerManager from the core crate to enable:
//! - Text search with visual highlighting
//! - Mouse-based text selection
//! - Clipboard copy operations
//!
//! Text selection works in two modes:
//! 1. **Click-and-drag**: Select text by dragging over it with the mouse
//! 2. **Search highlights**: Highlight all instances of a search query

use pdf_editor_core::annotation::PageCoordinate;
use pdf_editor_core::text_edit::TextEditManager;
use pdf_editor_core::text_layer::{SearchMatch, TextBoundingBox, TextLayerManager, TextSpan};
use std::sync::Arc;

/// Text selection state
#[derive(Debug, Clone)]
pub struct TextSelection {
    /// Page index of the selection
    pub page_index: u16,

    /// Selection rectangle in page coordinates
    pub selection_rect: TextBoundingBox,

    /// Start position in page coordinates
    pub start_point: PageCoordinate,

    /// End position in page coordinates (current mouse position)
    pub end_point: PageCoordinate,

    /// Whether the selection is active (user is dragging)
    pub is_active: bool,

    /// Selected text content
    pub text: String,
}

impl TextSelection {
    /// Create a new text selection starting at a point
    pub fn new(page_index: u16, start_point: PageCoordinate) -> Self {
        Self {
            page_index,
            selection_rect: TextBoundingBox::new(start_point.x, start_point.y, 0.0, 0.0),
            start_point,
            end_point: start_point,
            is_active: true,
            text: String::new(),
        }
    }

    /// Update the selection end point and recompute the selection rectangle
    pub fn update_end_point(&mut self, end_point: PageCoordinate) {
        self.end_point = end_point;

        // Calculate selection rectangle (normalized min/max)
        let min_x = self.start_point.x.min(end_point.x);
        let min_y = self.start_point.y.min(end_point.y);
        let max_x = self.start_point.x.max(end_point.x);
        let max_y = self.start_point.y.max(end_point.y);

        self.selection_rect = TextBoundingBox::new(min_x, min_y, max_x - min_x, max_y - min_y);
    }

    /// Finalize the selection (user released mouse)
    pub fn finalize(&mut self) {
        self.is_active = false;
    }

    /// Check if the selection is empty (no area)
    pub fn is_empty(&self) -> bool {
        self.selection_rect.width < 1.0 || self.selection_rect.height < 1.0
    }

    /// Get bounding boxes for rendering (in page coordinates)
    pub fn get_highlight_boxes(&self) -> Vec<TextBoundingBox> {
        vec![self.selection_rect]
    }
}

/// Search result with visual highlighting
#[derive(Debug, Clone)]
pub struct SearchResult {
    /// Page index containing the match
    pub page_index: u16,

    /// Match details
    pub search_match: SearchMatch,

    /// Whether this result is currently selected/active
    pub is_active: bool,
}

impl SearchResult {
    /// Create a new search result
    pub fn new(page_index: u16, search_match: SearchMatch) -> Self {
        Self {
            page_index,
            search_match,
            is_active: false,
        }
    }

    /// Get the bounding box for highlighting
    pub fn get_highlight_box(&self) -> TextBoundingBox {
        self.search_match.bbox
    }
}

/// Text search and selection manager
///
/// Manages text search queries, search results, and text selection state.
/// Works with TextLayerManager to query text content and coordinates.
/// Also supports text edits for selection and copy operations.
pub struct TextSearchManager {
    /// Reference to the document's text layer manager
    text_layers: Arc<TextLayerManager>,

    /// Reference to the text edit manager (optional)
    text_edits: Option<Arc<TextEditManager>>,

    /// Current search query (if any)
    search_query: Option<String>,

    /// Search results from the current query
    search_results: Vec<SearchResult>,

    /// Currently selected search result index
    selected_result_index: Option<usize>,

    /// Active text selection (if user is selecting text with mouse)
    text_selection: Option<TextSelection>,
}

impl TextSearchManager {
    /// Create a new text search manager
    pub fn new(text_layers: Arc<TextLayerManager>) -> Self {
        Self {
            text_layers,
            text_edits: None,
            search_query: None,
            search_results: Vec::new(),
            selected_result_index: None,
            text_selection: None,
        }
    }

    /// Set the text edit manager
    ///
    /// This enables text selection to also query text edits for selectable content.
    pub fn set_text_edit_manager(&mut self, text_edits: Arc<TextEditManager>) {
        self.text_edits = Some(text_edits);
    }

    /// Start a new search query
    ///
    /// Searches all pages with text layers and collects all matches.
    /// Returns the number of matches found.
    pub fn search(&mut self, query: &str) -> usize {
        if query.is_empty() {
            self.clear_search();
            return 0;
        }

        self.search_query = Some(query.to_string());
        self.search_results.clear();
        self.selected_result_index = None;

        // Search all pages
        let page_results = self.text_layers.search_all(query);

        // Flatten results into SearchResult objects
        for (page_index, matches) in page_results {
            for search_match in matches {
                self.search_results
                    .push(SearchResult::new(page_index, search_match));
            }
        }

        // Select first result if any
        if !self.search_results.is_empty() {
            self.selected_result_index = Some(0);
            self.search_results[0].is_active = true;
        }

        self.search_results.len()
    }

    /// Clear the current search
    pub fn clear_search(&mut self) {
        self.search_query = None;
        self.search_results.clear();
        self.selected_result_index = None;
    }

    /// Navigate to the next search result
    ///
    /// Returns the page index and bounding box to navigate to, or None if no results.
    pub fn next_result(&mut self) -> Option<(u16, TextBoundingBox)> {
        if self.search_results.is_empty() {
            return None;
        }

        // Deactivate current result
        if let Some(current) = self.selected_result_index {
            self.search_results[current].is_active = false;
        }

        // Move to next result (wrapping around)
        let next_index = match self.selected_result_index {
            Some(idx) => (idx + 1) % self.search_results.len(),
            None => 0,
        };

        self.selected_result_index = Some(next_index);
        self.search_results[next_index].is_active = true;

        let result = &self.search_results[next_index];
        Some((result.page_index, result.get_highlight_box()))
    }

    /// Navigate to the previous search result
    ///
    /// Returns the page index and bounding box to navigate to, or None if no results.
    pub fn previous_result(&mut self) -> Option<(u16, TextBoundingBox)> {
        if self.search_results.is_empty() {
            return None;
        }

        // Deactivate current result
        if let Some(current) = self.selected_result_index {
            self.search_results[current].is_active = false;
        }

        // Move to previous result (wrapping around)
        let prev_index = match self.selected_result_index {
            Some(idx) => {
                if idx == 0 {
                    self.search_results.len() - 1
                } else {
                    idx - 1
                }
            }
            None => self.search_results.len() - 1,
        };

        self.selected_result_index = Some(prev_index);
        self.search_results[prev_index].is_active = true;

        let result = &self.search_results[prev_index];
        Some((result.page_index, result.get_highlight_box()))
    }

    /// Get all search results for a specific page
    ///
    /// Used for rendering search highlights on the current page.
    pub fn get_results_for_page(&self, page_index: u16) -> Vec<&SearchResult> {
        self.search_results
            .iter()
            .filter(|r| r.page_index == page_index)
            .collect()
    }

    /// Get the currently active search result
    pub fn get_active_result(&self) -> Option<&SearchResult> {
        self.selected_result_index
            .and_then(|idx| self.search_results.get(idx))
    }

    /// Get the total number of search results
    pub fn result_count(&self) -> usize {
        self.search_results.len()
    }

    /// Get the current search query
    pub fn query(&self) -> Option<&str> {
        self.search_query.as_deref()
    }

    /// Start a text selection at a point in page coordinates
    ///
    /// Called when the user clicks to start selecting text.
    pub fn start_selection(&mut self, page_index: u16, point: PageCoordinate) {
        self.text_selection = Some(TextSelection::new(page_index, point));
    }

    /// Update the text selection end point
    ///
    /// Called as the user drags to extend the selection.
    pub fn update_selection(&mut self, end_point: PageCoordinate) {
        if let Some(selection) = &mut self.text_selection {
            selection.update_end_point(end_point);

            let mut text_parts = Vec::new();

            // Query the text layer for text in the selection rectangle
            if let Some(layer) = self.text_layers.get_layer(selection.page_index) {
                let spans = layer.find_spans_in_rect(&selection.selection_rect);
                for span in spans {
                    text_parts.push(span.text.clone());
                }
            }

            // Also query text edits in the selection rectangle
            if let Some(ref text_edits) = self.text_edits {
                if let Ok(edits) = text_edits.get_page_edits(selection.page_index) {
                    for edit in edits {
                        if edit.visible && edit.overlaps(&selection.selection_rect) {
                            text_parts.push(edit.edited_text.clone());
                        }
                    }
                }
            }

            selection.text = text_parts.join(" ");
        }
    }

    /// Finalize the text selection
    ///
    /// Called when the user releases the mouse button.
    /// Returns the selected text if any.
    pub fn end_selection(&mut self) -> Option<String> {
        if let Some(selection) = &mut self.text_selection {
            selection.finalize();

            if selection.is_empty() || selection.text.is_empty() {
                self.text_selection = None;
                return None;
            }

            Some(selection.text.clone())
        } else {
            None
        }
    }

    /// Clear the current text selection
    pub fn clear_selection(&mut self) {
        self.text_selection = None;
    }

    /// Get the current text selection
    pub fn get_selection(&self) -> Option<&TextSelection> {
        self.text_selection.as_ref()
    }

    /// Get the selected text (if any)
    pub fn get_selected_text(&self) -> Option<&str> {
        self.text_selection.as_ref().map(|s| s.text.as_str())
    }

    /// Select a word at a specific point (for double-click)
    ///
    /// Finds the text span containing the point and selects its text.
    /// Returns the selected text if successful.
    pub fn select_word_at_point(&mut self, page_index: u16, point: PageCoordinate) -> Option<String> {
        if let Some(layer) = self.text_layers.get_layer(page_index) {
            if let Some(span) = layer.find_span_at_point(&point) {
                // Create selection from the span's bounding box
                let mut selection = TextSelection::new(page_index, PageCoordinate::new(span.bbox.x, span.bbox.y));
                selection.update_end_point(PageCoordinate::new(
                    span.bbox.x + span.bbox.width,
                    span.bbox.y + span.bbox.height,
                ));
                selection.text = span.text.clone();
                selection.finalize();
                let text = selection.text.clone();
                self.text_selection = Some(selection);
                return Some(text);
            }
        }
        None
    }

    /// Select a line at a specific point (for triple-click)
    ///
    /// Finds all text spans on the same horizontal line as the point and selects them.
    /// Returns the selected text if successful.
    pub fn select_line_at_point(&mut self, page_index: u16, point: PageCoordinate) -> Option<String> {
        if let Some(layer) = self.text_layers.get_layer(page_index) {
            // Find the span at the point to determine the line's Y position
            if let Some(target_span) = layer.find_span_at_point(&point) {
                // Get the vertical center of the target span
                let target_y_center = target_span.bbox.y + target_span.bbox.height / 2.0;

                // Find all spans that overlap vertically with the target span
                // A span is on the same line if its vertical center is within the target's height
                let line_spans: Vec<&TextSpan> = layer.spans.iter()
                    .filter(|span| {
                        let span_y_center = span.bbox.y + span.bbox.height / 2.0;
                        // Check if spans are on the same line (within reasonable tolerance)
                        let tolerance = target_span.bbox.height * 0.5;
                        (span_y_center - target_y_center).abs() < tolerance
                    })
                    .collect();

                if !line_spans.is_empty() {
                    // Calculate bounding box for all spans on the line
                    let min_x = line_spans.iter().map(|s| s.bbox.x).fold(f32::INFINITY, f32::min);
                    let min_y = line_spans.iter().map(|s| s.bbox.y).fold(f32::INFINITY, f32::min);
                    let max_x = line_spans.iter().map(|s| s.bbox.x + s.bbox.width).fold(f32::NEG_INFINITY, f32::max);
                    let max_y = line_spans.iter().map(|s| s.bbox.y + s.bbox.height).fold(f32::NEG_INFINITY, f32::max);

                    // Create selection from the combined bounding box
                    let mut selection = TextSelection::new(page_index, PageCoordinate::new(min_x, min_y));
                    selection.update_end_point(PageCoordinate::new(max_x, max_y));

                    // Collect text from all spans on the line, sorted by x position
                    let mut sorted_spans: Vec<_> = line_spans.iter().collect();
                    sorted_spans.sort_by(|a, b| a.bbox.x.partial_cmp(&b.bbox.x).unwrap_or(std::cmp::Ordering::Equal));
                    selection.text = sorted_spans.iter().map(|s| s.text.as_str()).collect::<Vec<_>>().join(" ");
                    selection.finalize();

                    let text = selection.text.clone();
                    self.text_selection = Some(selection);
                    return Some(text);
                }
            }
        }
        None
    }

    /// Get highlight boxes for the current page
    ///
    /// Returns both search result highlights and text selection highlights.
    /// Used by the compositor to render highlights on the page.
    pub fn get_highlights_for_page(&self, page_index: u16) -> Vec<HighlightBox> {
        let mut highlights = Vec::new();

        // Add search result highlights
        for result in self.get_results_for_page(page_index) {
            highlights.push(HighlightBox {
                bbox: result.get_highlight_box(),
                highlight_type: if result.is_active {
                    HighlightType::ActiveSearch
                } else {
                    HighlightType::Search
                },
            });
        }

        // Add text selection highlight
        if let Some(selection) = &self.text_selection {
            if selection.page_index == page_index && !selection.is_empty() {
                for bbox in selection.get_highlight_boxes() {
                    highlights.push(HighlightBox {
                        bbox,
                        highlight_type: HighlightType::Selection,
                    });
                }
            }
        }

        highlights
    }
}

/// A highlight box to render on the page
#[derive(Debug, Clone)]
pub struct HighlightBox {
    /// Bounding box in page coordinates
    pub bbox: TextBoundingBox,

    /// Type of highlight (determines color/style)
    pub highlight_type: HighlightType,
}

/// Type of text highlight
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HighlightType {
    /// Search result (inactive)
    Search,

    /// Active search result (currently selected)
    ActiveSearch,

    /// User text selection
    Selection,
}

impl HighlightType {
    /// Get the color for this highlight type (RGBA, normalized 0-1)
    pub fn color(&self) -> (f32, f32, f32, f32) {
        match self {
            HighlightType::Search => (1.0, 1.0, 0.0, 0.3), // Yellow, semi-transparent
            HighlightType::ActiveSearch => (1.0, 0.65, 0.0, 0.5), // Orange, more opaque
            HighlightType::Selection => (0.2, 0.6, 1.0, 0.3), // Blue, semi-transparent
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pdf_editor_core::text_layer::{PageTextLayer, TextSpan};

    fn create_test_manager() -> TextSearchManager {
        let manager = TextLayerManager::new(2);

        // Create test text layer for page 0
        let spans = vec![
            TextSpan::new(
                "Hello world".to_string(),
                TextBoundingBox::new(10.0, 10.0, 100.0, 20.0),
                0.9,
                12.0,
            ),
            TextSpan::new(
                "This is a test".to_string(),
                TextBoundingBox::new(10.0, 40.0, 120.0, 20.0),
                0.9,
                12.0,
            ),
        ];
        let layer = PageTextLayer::from_spans(0, spans);
        manager.set_layer(layer);

        TextSearchManager::new(Arc::new(manager))
    }

    #[test]
    fn test_search_basic() {
        let mut search_mgr = create_test_manager();

        // Search for "test"
        let count = search_mgr.search("test");
        assert_eq!(count, 1);
        assert_eq!(search_mgr.result_count(), 1);
    }

    #[test]
    fn test_search_case_insensitive() {
        let mut search_mgr = create_test_manager();

        // Search for "HELLO" (should match "Hello")
        let count = search_mgr.search("HELLO");
        assert_eq!(count, 1);
    }

    #[test]
    fn test_search_multiple_results() {
        let mut search_mgr = create_test_manager();

        // Search for "is" (should match "This is")
        let count = search_mgr.search("is");
        assert_eq!(count, 2); // "This" and "is"
    }

    #[test]
    fn test_search_navigation() {
        let mut search_mgr = create_test_manager();

        search_mgr.search("is");
        assert_eq!(search_mgr.result_count(), 2);

        // Navigate to next result
        let result1 = search_mgr.next_result();
        assert!(result1.is_some());

        // Navigate to next result (should wrap around)
        let result2 = search_mgr.next_result();
        assert!(result2.is_some());

        // Navigate to previous result
        let result3 = search_mgr.previous_result();
        assert!(result3.is_some());
    }

    #[test]
    fn test_clear_search() {
        let mut search_mgr = create_test_manager();

        search_mgr.search("test");
        assert_eq!(search_mgr.result_count(), 1);

        search_mgr.clear_search();
        assert_eq!(search_mgr.result_count(), 0);
        assert!(search_mgr.query().is_none());
    }

    #[test]
    fn test_text_selection_basic() {
        let mut search_mgr = create_test_manager();

        // Start selection
        search_mgr.start_selection(0, PageCoordinate::new(10.0, 10.0));
        assert!(search_mgr.get_selection().is_some());

        // Update selection
        search_mgr.update_selection(PageCoordinate::new(110.0, 30.0));
        let selection = search_mgr.get_selection().unwrap();
        assert!(!selection.is_empty());

        // End selection
        let selected_text = search_mgr.end_selection();
        assert!(selected_text.is_some());
        assert!(!selected_text.unwrap().is_empty());
    }

    #[test]
    fn test_text_selection_empty() {
        let mut search_mgr = create_test_manager();

        // Start and end selection at same point (empty selection)
        search_mgr.start_selection(0, PageCoordinate::new(10.0, 10.0));
        let selected_text = search_mgr.end_selection();
        assert!(selected_text.is_none());
        assert!(search_mgr.get_selection().is_none());
    }

    #[test]
    fn test_highlights_for_page() {
        let mut search_mgr = create_test_manager();

        // Add search results
        search_mgr.search("is");

        // Get highlights for page 0
        let highlights = search_mgr.get_highlights_for_page(0);
        assert_eq!(highlights.len(), 2); // Two search results

        // Check highlight types
        assert_eq!(highlights[0].highlight_type, HighlightType::ActiveSearch);
        assert_eq!(highlights[1].highlight_type, HighlightType::Search);
    }

    #[test]
    fn test_highlight_colors() {
        let search_color = HighlightType::Search.color();
        let active_color = HighlightType::ActiveSearch.color();
        let selection_color = HighlightType::Selection.color();

        // Verify colors are different
        assert_ne!(search_color, active_color);
        assert_ne!(search_color, selection_color);
        assert_ne!(active_color, selection_color);

        // Verify alpha channel (transparency)
        assert!(search_color.3 > 0.0 && search_color.3 < 1.0);
        assert!(active_color.3 > 0.0 && active_color.3 < 1.0);
        assert!(selection_color.3 > 0.0 && selection_color.3 < 1.0);
    }

    #[test]
    fn test_select_word_at_point() {
        let mut search_mgr = create_test_manager();

        // Select word "Hello" by clicking on it
        let text = search_mgr.select_word_at_point(0, PageCoordinate::new(50.0, 20.0));
        assert!(text.is_some());
        assert_eq!(text.unwrap(), "Hello world");

        // Verify selection state
        let selection = search_mgr.get_selection().unwrap();
        assert_eq!(selection.page_index, 0);
        assert!(!selection.is_active); // Finalized
        assert!(!selection.text.is_empty());
    }

    #[test]
    fn test_select_word_at_point_outside_text() {
        let mut search_mgr = create_test_manager();

        // Try to select word at a point with no text
        let text = search_mgr.select_word_at_point(0, PageCoordinate::new(500.0, 500.0));
        assert!(text.is_none());
        assert!(search_mgr.get_selection().is_none());
    }

    #[test]
    fn test_select_line_at_point() {
        let mut search_mgr = create_test_manager();

        // Select line by clicking on "Hello" (first line)
        let text = search_mgr.select_line_at_point(0, PageCoordinate::new(50.0, 20.0));
        assert!(text.is_some());

        // Should select "Hello world" (first span)
        let selected = text.unwrap();
        assert!(!selected.is_empty());

        // Verify selection state
        let selection = search_mgr.get_selection().unwrap();
        assert_eq!(selection.page_index, 0);
        assert!(!selection.is_active); // Finalized
    }

    #[test]
    fn test_select_line_at_point_outside_text() {
        let mut search_mgr = create_test_manager();

        // Try to select line at a point with no text
        let text = search_mgr.select_line_at_point(0, PageCoordinate::new(500.0, 500.0));
        assert!(text.is_none());
        assert!(search_mgr.get_selection().is_none());
    }

    fn create_multiline_test_manager() -> TextSearchManager {
        let manager = TextLayerManager::new(2);

        // Create test text layer with multiple lines on page 0
        let spans = vec![
            // Line 1: y = 10-30
            TextSpan::new(
                "First".to_string(),
                TextBoundingBox::new(10.0, 10.0, 50.0, 20.0),
                0.9,
                12.0,
            ),
            TextSpan::new(
                "line".to_string(),
                TextBoundingBox::new(70.0, 10.0, 40.0, 20.0),
                0.9,
                12.0,
            ),
            TextSpan::new(
                "here".to_string(),
                TextBoundingBox::new(120.0, 10.0, 40.0, 20.0),
                0.9,
                12.0,
            ),
            // Line 2: y = 40-60
            TextSpan::new(
                "Second".to_string(),
                TextBoundingBox::new(10.0, 40.0, 60.0, 20.0),
                0.9,
                12.0,
            ),
            TextSpan::new(
                "line".to_string(),
                TextBoundingBox::new(80.0, 40.0, 40.0, 20.0),
                0.9,
                12.0,
            ),
        ];
        let layer = PageTextLayer::from_spans(0, spans);
        manager.set_layer(layer);

        TextSearchManager::new(Arc::new(manager))
    }

    #[test]
    fn test_select_line_multiline() {
        let mut search_mgr = create_multiline_test_manager();

        // Select first line by clicking on "First"
        let text = search_mgr.select_line_at_point(0, PageCoordinate::new(30.0, 20.0));
        assert!(text.is_some());
        let selected = text.unwrap();
        // Should contain all words from first line
        assert!(selected.contains("First"));
        assert!(selected.contains("line"));
        assert!(selected.contains("here"));
        // Should NOT contain words from second line
        assert!(!selected.contains("Second"));

        // Select second line by clicking on "Second"
        let text2 = search_mgr.select_line_at_point(0, PageCoordinate::new(30.0, 50.0));
        assert!(text2.is_some());
        let selected2 = text2.unwrap();
        // Should contain all words from second line
        assert!(selected2.contains("Second"));
        assert!(selected2.contains("line"));
        // Should NOT contain words from first line
        assert!(!selected2.contains("First"));
        assert!(!selected2.contains("here"));
    }
}
