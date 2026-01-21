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
use pdf_editor_core::text_layer::{SearchMatch, TextBoundingBox, TextLayerManager};
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

        self.selection_rect = TextBoundingBox::new(
            min_x,
            min_y,
            max_x - min_x,
            max_y - min_y,
        );
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
pub struct TextSearchManager {
    /// Reference to the document's text layer manager
    text_layers: Arc<TextLayerManager>,

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
            search_query: None,
            search_results: Vec::new(),
            selected_result_index: None,
            text_selection: None,
        }
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
                self.search_results.push(SearchResult::new(page_index, search_match));
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
        self.selected_result_index.and_then(|idx| self.search_results.get(idx))
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

            // Query the text layer for text in the selection rectangle
            if let Some(layer) = self.text_layers.get_layer(selection.page_index) {
                let spans = layer.find_spans_in_rect(&selection.selection_rect);
                selection.text = spans
                    .iter()
                    .map(|span| span.text.as_str())
                    .collect::<Vec<_>>()
                    .join(" ");
            }
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
            HighlightType::Search => (1.0, 1.0, 0.0, 0.3),      // Yellow, semi-transparent
            HighlightType::ActiveSearch => (1.0, 0.65, 0.0, 0.5), // Orange, more opaque
            HighlightType::Selection => (0.2, 0.6, 1.0, 0.3),    // Blue, semi-transparent
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
}
