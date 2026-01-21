//! Progressive OCR system
//!
//! Manages progressive OCR scheduling: current page → nearby pages → remaining pages when idle.
//! This module coordinates with the job scheduler to submit OCR jobs in the optimal order
//! for the best user experience.

use crate::ocr::{OcrEngine, OcrResult};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

/// Progressive OCR scheduler state
#[derive(Debug)]
struct ProgressiveOcrState {
    /// OCR engine
    engine: Arc<OcrEngine>,

    /// Current page index (highest priority)
    current_page: u16,

    /// Total number of pages in the document
    total_pages: u16,

    /// Pages that have already been processed or are in progress
    processed_pages: HashSet<u16>,

    /// Pages that are currently being processed
    in_progress_pages: HashSet<u16>,

    /// Pages that need OCR (detected as having no text)
    pages_needing_ocr: HashSet<u16>,

    /// OCR results cache
    results_cache: HashMap<u16, OcrResult>,

    /// Whether progressive OCR is enabled
    enabled: bool,

    /// Maximum number of concurrent OCR operations
    /// (stored for future use in integration with worker pool)
    #[allow(dead_code)]
    max_concurrent: usize,
}

/// Progressive OCR scheduler
///
/// This scheduler manages OCR operations across a PDF document in a progressive manner:
/// 1. Current page (highest priority)
/// 2. Nearby pages (adjacent to current)
/// 3. Remaining pages (when system is idle)
///
/// The scheduler integrates with the job scheduler to submit OCR jobs at the appropriate
/// priority level (JobPriority::Ocr, which is the lowest priority).
///
/// # Example
///
/// ```
/// use pdf_editor_core::{OcrEngine, OcrConfig, ProgressiveOcr};
///
/// let ocr_engine = OcrEngine::new(OcrConfig::default()).unwrap();
/// ocr_engine.initialize().unwrap();
///
/// let mut progressive_ocr = ProgressiveOcr::new(ocr_engine, 10);
///
/// // Set current page (triggers OCR for this page first)
/// progressive_ocr.set_current_page(0);
///
/// // Get next pages to process
/// let pages_to_process = progressive_ocr.get_next_pages_to_process(5);
/// for page_index in pages_to_process {
///     // Submit OCR job to scheduler
///     println!("Processing page {}", page_index);
/// }
/// ```
pub struct ProgressiveOcr {
    state: Arc<Mutex<ProgressiveOcrState>>,
}

impl ProgressiveOcr {
    /// Create a new progressive OCR scheduler
    ///
    /// # Arguments
    ///
    /// * `engine` - OCR engine to use for processing
    /// * `total_pages` - Total number of pages in the document
    pub fn new(engine: OcrEngine, total_pages: u16) -> Self {
        let config = engine.config();
        let state = ProgressiveOcrState {
            engine: Arc::new(engine),
            current_page: 0,
            total_pages,
            processed_pages: HashSet::new(),
            in_progress_pages: HashSet::new(),
            pages_needing_ocr: HashSet::new(),
            results_cache: HashMap::new(),
            enabled: config.progressive,
            max_concurrent: config.max_concurrent,
        };

        Self {
            state: Arc::new(Mutex::new(state)),
        }
    }

    /// Set the current page
    ///
    /// This updates the priority for OCR processing. The current page
    /// will be processed first, followed by nearby pages.
    pub fn set_current_page(&self, page_index: u16) {
        let mut state = self.state.lock().unwrap();
        if page_index < state.total_pages {
            state.current_page = page_index;
        }
    }

    /// Get the current page
    pub fn current_page(&self) -> u16 {
        let state = self.state.lock().unwrap();
        state.current_page
    }

    /// Mark a page as needing OCR
    ///
    /// This should be called after detecting that a page has no selectable text.
    /// The page will be added to the queue for OCR processing.
    pub fn mark_page_needs_ocr(&self, page_index: u16) {
        let mut state = self.state.lock().unwrap();
        if page_index < state.total_pages && !state.processed_pages.contains(&page_index) {
            state.pages_needing_ocr.insert(page_index);
        }
    }

    /// Mark a page as not needing OCR
    ///
    /// This should be called when a page is detected to have sufficient selectable text.
    pub fn mark_page_has_text(&self, page_index: u16) {
        let mut state = self.state.lock().unwrap();
        state.pages_needing_ocr.remove(&page_index);
        state.processed_pages.insert(page_index);
    }

    /// Get the next pages to process
    ///
    /// Returns a list of up to `count` page indices that should be processed next,
    /// ordered by priority:
    /// 1. Current page (if needs OCR)
    /// 2. Nearby pages (adjacent to current)
    /// 3. Remaining pages (in order)
    ///
    /// Only returns pages that need OCR and are not already being processed.
    ///
    /// # Arguments
    ///
    /// * `count` - Maximum number of pages to return
    pub fn get_next_pages_to_process(&self, count: usize) -> Vec<u16> {
        let state = self.state.lock().unwrap();

        if !state.enabled {
            return Vec::new();
        }

        let mut pages = Vec::new();
        let current = state.current_page;

        // Helper to check if we should process a page
        let should_process = |page: u16| -> bool {
            page < state.total_pages
                && state.pages_needing_ocr.contains(&page)
                && !state.in_progress_pages.contains(&page)
                && !state.processed_pages.contains(&page)
        };

        // Priority 1: Current page
        if should_process(current) {
            pages.push(current);
            if pages.len() >= count {
                return pages;
            }
        }

        // Priority 2: Nearby pages (adjacent pages in expanding radius)
        // We process: current-1, current+1, current-2, current+2, ...
        let mut radius = 1u16;
        while pages.len() < count && radius < state.total_pages {
            // Page before current
            if current >= radius {
                let page = current - radius;
                if should_process(page) {
                    pages.push(page);
                    if pages.len() >= count {
                        return pages;
                    }
                }
            }

            // Page after current
            let page = current.saturating_add(radius);
            if should_process(page) {
                pages.push(page);
                if pages.len() >= count {
                    return pages;
                }
            }

            radius += 1;
        }

        // Priority 3: Remaining pages (in order)
        for page in 0..state.total_pages {
            if should_process(page) && !pages.contains(&page) {
                pages.push(page);
                if pages.len() >= count {
                    break;
                }
            }
        }

        pages
    }

    /// Mark a page as being processed
    ///
    /// This should be called when an OCR job starts processing a page.
    pub fn mark_page_in_progress(&self, page_index: u16) {
        let mut state = self.state.lock().unwrap();
        if page_index < state.total_pages {
            state.in_progress_pages.insert(page_index);
        }
    }

    /// Mark a page as processed and store the result
    ///
    /// This should be called when an OCR job completes.
    pub fn mark_page_complete(&self, page_index: u16, result: OcrResult) {
        let mut state = self.state.lock().unwrap();
        if page_index < state.total_pages {
            state.in_progress_pages.remove(&page_index);
            state.processed_pages.insert(page_index);
            state.pages_needing_ocr.remove(&page_index);
            state.results_cache.insert(page_index, result);
        }
    }

    /// Mark a page as failed
    ///
    /// This should be called when an OCR job fails. The page will be removed
    /// from in-progress but not marked as processed, allowing it to be retried.
    pub fn mark_page_failed(&self, page_index: u16) {
        let mut state = self.state.lock().unwrap();
        state.in_progress_pages.remove(&page_index);
    }

    /// Get OCR result for a page
    ///
    /// Returns the cached OCR result if available.
    pub fn get_result(&self, page_index: u16) -> Option<OcrResult> {
        let state = self.state.lock().unwrap();
        state.results_cache.get(&page_index).cloned()
    }

    /// Check if a page has been processed
    pub fn is_page_processed(&self, page_index: u16) -> bool {
        let state = self.state.lock().unwrap();
        state.processed_pages.contains(&page_index)
    }

    /// Check if a page is currently being processed
    pub fn is_page_in_progress(&self, page_index: u16) -> bool {
        let state = self.state.lock().unwrap();
        state.in_progress_pages.contains(&page_index)
    }

    /// Check if a page needs OCR
    pub fn page_needs_ocr(&self, page_index: u16) -> bool {
        let state = self.state.lock().unwrap();
        state.pages_needing_ocr.contains(&page_index)
    }

    /// Get the number of pages that need OCR
    pub fn pages_needing_ocr_count(&self) -> usize {
        let state = self.state.lock().unwrap();
        state.pages_needing_ocr.len()
    }

    /// Get the number of pages that have been processed
    pub fn pages_processed_count(&self) -> usize {
        let state = self.state.lock().unwrap();
        state.processed_pages.len()
    }

    /// Get the number of pages currently being processed
    pub fn pages_in_progress_count(&self) -> usize {
        let state = self.state.lock().unwrap();
        state.in_progress_pages.len()
    }

    /// Check if all pages that need OCR have been processed
    pub fn is_complete(&self) -> bool {
        let state = self.state.lock().unwrap();
        state.pages_needing_ocr.is_empty() && state.in_progress_pages.is_empty()
    }

    /// Enable or disable progressive OCR
    pub fn set_enabled(&self, enabled: bool) {
        let mut state = self.state.lock().unwrap();
        state.enabled = enabled;
    }

    /// Check if progressive OCR is enabled
    pub fn is_enabled(&self) -> bool {
        let state = self.state.lock().unwrap();
        state.enabled
    }

    /// Get the OCR engine
    pub fn engine(&self) -> Arc<OcrEngine> {
        let state = self.state.lock().unwrap();
        Arc::clone(&state.engine)
    }

    /// Reset the progressive OCR state
    ///
    /// Clears all processed pages, in-progress pages, and results cache.
    /// Useful when reloading a document or switching documents.
    pub fn reset(&self) {
        let mut state = self.state.lock().unwrap();
        state.processed_pages.clear();
        state.in_progress_pages.clear();
        state.pages_needing_ocr.clear();
        state.results_cache.clear();
        state.current_page = 0;
    }

    /// Get statistics about OCR progress
    pub fn stats(&self) -> ProgressiveOcrStats {
        let state = self.state.lock().unwrap();
        ProgressiveOcrStats {
            total_pages: state.total_pages,
            pages_needing_ocr: state.pages_needing_ocr.len(),
            pages_processed: state.processed_pages.len(),
            pages_in_progress: state.in_progress_pages.len(),
            current_page: state.current_page,
            enabled: state.enabled,
        }
    }
}

/// Statistics about progressive OCR progress
#[derive(Debug, Clone)]
pub struct ProgressiveOcrStats {
    /// Total number of pages in the document
    pub total_pages: u16,

    /// Number of pages that need OCR
    pub pages_needing_ocr: usize,

    /// Number of pages that have been processed
    pub pages_processed: usize,

    /// Number of pages currently being processed
    pub pages_in_progress: usize,

    /// Current page index
    pub current_page: u16,

    /// Whether progressive OCR is enabled
    pub enabled: bool,
}

impl ProgressiveOcrStats {
    /// Get the percentage of pages processed (0.0 to 1.0)
    pub fn progress(&self) -> f32 {
        let total_to_process = self.pages_needing_ocr + self.pages_processed;
        if total_to_process == 0 {
            return 1.0;
        }
        self.pages_processed as f32 / total_to_process as f32
    }

    /// Check if OCR is complete
    pub fn is_complete(&self) -> bool {
        self.pages_needing_ocr == 0 && self.pages_in_progress == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ocr::OcrConfig;

    fn create_test_engine() -> OcrEngine {
        OcrEngine::new(OcrConfig::default()).unwrap()
    }

    #[test]
    fn test_progressive_ocr_creation() {
        let engine = create_test_engine();
        let progressive = ProgressiveOcr::new(engine, 10);

        assert_eq!(progressive.current_page(), 0);
        assert_eq!(progressive.pages_needing_ocr_count(), 0);
        assert_eq!(progressive.pages_processed_count(), 0);
        assert!(progressive.is_enabled());
    }

    #[test]
    fn test_set_current_page() {
        let engine = create_test_engine();
        let progressive = ProgressiveOcr::new(engine, 10);

        progressive.set_current_page(5);
        assert_eq!(progressive.current_page(), 5);

        // Out of bounds should be ignored
        progressive.set_current_page(15);
        assert_eq!(progressive.current_page(), 5);
    }

    #[test]
    fn test_mark_page_needs_ocr() {
        let engine = create_test_engine();
        let progressive = ProgressiveOcr::new(engine, 10);

        progressive.mark_page_needs_ocr(0);
        progressive.mark_page_needs_ocr(5);

        assert_eq!(progressive.pages_needing_ocr_count(), 2);
        assert!(progressive.page_needs_ocr(0));
        assert!(progressive.page_needs_ocr(5));
        assert!(!progressive.page_needs_ocr(1));
    }

    #[test]
    fn test_mark_page_has_text() {
        let engine = create_test_engine();
        let progressive = ProgressiveOcr::new(engine, 10);

        progressive.mark_page_needs_ocr(0);
        assert!(progressive.page_needs_ocr(0));

        progressive.mark_page_has_text(0);
        assert!(!progressive.page_needs_ocr(0));
        assert!(progressive.is_page_processed(0));
    }

    #[test]
    fn test_get_next_pages_to_process_current_page() {
        let engine = create_test_engine();
        let progressive = ProgressiveOcr::new(engine, 10);

        progressive.set_current_page(5);
        progressive.mark_page_needs_ocr(5);

        let pages = progressive.get_next_pages_to_process(1);
        assert_eq!(pages, vec![5]);
    }

    #[test]
    fn test_get_next_pages_to_process_nearby_pages() {
        let engine = create_test_engine();
        let progressive = ProgressiveOcr::new(engine, 10);

        progressive.set_current_page(5);
        for i in 0..10 {
            progressive.mark_page_needs_ocr(i);
        }

        let pages = progressive.get_next_pages_to_process(5);
        // Should get: 5 (current), 4 (current-1), 6 (current+1), 3 (current-2), 7 (current+2)
        assert_eq!(pages, vec![5, 4, 6, 3, 7]);
    }

    #[test]
    fn test_get_next_pages_to_process_remaining_pages() {
        let engine = create_test_engine();
        let progressive = ProgressiveOcr::new(engine, 10);

        progressive.set_current_page(5);
        // Mark pages 0, 1, 2 as needing OCR (not nearby)
        progressive.mark_page_needs_ocr(0);
        progressive.mark_page_needs_ocr(1);
        progressive.mark_page_needs_ocr(2);

        let pages = progressive.get_next_pages_to_process(10);
        // Should include 0, 1, 2 in order after checking nearby pages
        assert!(pages.contains(&0));
        assert!(pages.contains(&1));
        assert!(pages.contains(&2));
    }

    #[test]
    fn test_get_next_pages_respects_in_progress() {
        let engine = create_test_engine();
        let progressive = ProgressiveOcr::new(engine, 10);

        progressive.set_current_page(5);
        progressive.mark_page_needs_ocr(5);
        progressive.mark_page_in_progress(5);

        let pages = progressive.get_next_pages_to_process(1);
        // Page 5 is in progress, should not be returned
        assert_eq!(pages, Vec::<u16>::new());
    }

    #[test]
    fn test_get_next_pages_respects_processed() {
        let engine = create_test_engine();
        let progressive = ProgressiveOcr::new(engine, 10);

        progressive.set_current_page(5);
        progressive.mark_page_needs_ocr(5);

        let result = OcrResult::new(5, "test".to_string(), Vec::new(), 0.9);
        progressive.mark_page_complete(5, result);

        let pages = progressive.get_next_pages_to_process(1);
        // Page 5 is processed, should not be returned
        assert_eq!(pages, Vec::<u16>::new());
    }

    #[test]
    fn test_mark_page_complete() {
        let engine = create_test_engine();
        let progressive = ProgressiveOcr::new(engine, 10);

        progressive.mark_page_needs_ocr(0);
        progressive.mark_page_in_progress(0);

        assert!(progressive.is_page_in_progress(0));
        assert!(!progressive.is_page_processed(0));

        let result = OcrResult::new(0, "test".to_string(), Vec::new(), 0.9);
        progressive.mark_page_complete(0, result.clone());

        assert!(!progressive.is_page_in_progress(0));
        assert!(progressive.is_page_processed(0));
        assert!(!progressive.page_needs_ocr(0));

        let cached = progressive.get_result(0).unwrap();
        assert_eq!(cached.page_index, 0);
        assert_eq!(cached.text, "test");
    }

    #[test]
    fn test_mark_page_failed() {
        let engine = create_test_engine();
        let progressive = ProgressiveOcr::new(engine, 10);

        progressive.mark_page_needs_ocr(0);
        progressive.mark_page_in_progress(0);

        assert!(progressive.is_page_in_progress(0));

        progressive.mark_page_failed(0);

        assert!(!progressive.is_page_in_progress(0));
        assert!(!progressive.is_page_processed(0));
        // Page still needs OCR (can be retried)
        assert!(progressive.page_needs_ocr(0));
    }

    #[test]
    fn test_is_complete() {
        let engine = create_test_engine();
        let progressive = ProgressiveOcr::new(engine, 10);

        assert!(progressive.is_complete());

        progressive.mark_page_needs_ocr(0);
        assert!(!progressive.is_complete());

        progressive.mark_page_in_progress(0);
        assert!(!progressive.is_complete());

        let result = OcrResult::new(0, "test".to_string(), Vec::new(), 0.9);
        progressive.mark_page_complete(0, result);
        assert!(progressive.is_complete());
    }

    #[test]
    fn test_enable_disable() {
        let engine = create_test_engine();
        let progressive = ProgressiveOcr::new(engine, 10);

        assert!(progressive.is_enabled());

        progressive.set_enabled(false);
        assert!(!progressive.is_enabled());

        // When disabled, should not return any pages to process
        progressive.mark_page_needs_ocr(0);
        let pages = progressive.get_next_pages_to_process(10);
        assert_eq!(pages.len(), 0);

        progressive.set_enabled(true);
        let pages = progressive.get_next_pages_to_process(10);
        assert_eq!(pages, vec![0]);
    }

    #[test]
    fn test_reset() {
        let engine = create_test_engine();
        let progressive = ProgressiveOcr::new(engine, 10);

        progressive.set_current_page(5);
        progressive.mark_page_needs_ocr(0);
        progressive.mark_page_in_progress(1);

        let result = OcrResult::new(2, "test".to_string(), Vec::new(), 0.9);
        progressive.mark_page_complete(2, result);

        assert_eq!(progressive.pages_needing_ocr_count(), 1);
        assert_eq!(progressive.pages_in_progress_count(), 1);
        assert_eq!(progressive.pages_processed_count(), 1);

        progressive.reset();

        assert_eq!(progressive.current_page(), 0);
        assert_eq!(progressive.pages_needing_ocr_count(), 0);
        assert_eq!(progressive.pages_in_progress_count(), 0);
        assert_eq!(progressive.pages_processed_count(), 0);
        assert!(progressive.get_result(2).is_none());
    }

    #[test]
    fn test_stats() {
        let engine = create_test_engine();
        let progressive = ProgressiveOcr::new(engine, 10);

        progressive.set_current_page(5);
        progressive.mark_page_needs_ocr(0);
        progressive.mark_page_needs_ocr(1);
        progressive.mark_page_needs_ocr(2);
        progressive.mark_page_in_progress(0);

        let result = OcrResult::new(1, "test".to_string(), Vec::new(), 0.9);
        progressive.mark_page_complete(1, result);

        let stats = progressive.stats();
        assert_eq!(stats.total_pages, 10);
        assert_eq!(stats.pages_needing_ocr, 2); // 0 and 2 (1 is processed)
        assert_eq!(stats.pages_in_progress, 1);
        assert_eq!(stats.pages_processed, 1);
        assert_eq!(stats.current_page, 5);
        assert!(stats.enabled);

        assert!(!stats.is_complete());
        assert_eq!(stats.progress(), 1.0 / 3.0); // 1 processed out of 3 total (0, 1, 2)
    }

    #[test]
    fn test_stats_progress() {
        let engine = create_test_engine();
        let progressive = ProgressiveOcr::new(engine, 10);

        let stats = progressive.stats();
        assert_eq!(stats.progress(), 1.0); // No pages need OCR

        progressive.mark_page_needs_ocr(0);
        progressive.mark_page_needs_ocr(1);

        let stats = progressive.stats();
        assert_eq!(stats.progress(), 0.0);

        let result = OcrResult::new(0, "test".to_string(), Vec::new(), 0.9);
        progressive.mark_page_complete(0, result);

        let stats = progressive.stats();
        assert_eq!(stats.progress(), 0.5);
    }

    #[test]
    fn test_get_next_pages_at_document_boundaries() {
        let engine = create_test_engine();
        let progressive = ProgressiveOcr::new(engine, 10);

        // Test at beginning of document
        progressive.set_current_page(0);
        progressive.mark_page_needs_ocr(0);
        progressive.mark_page_needs_ocr(1);
        progressive.mark_page_needs_ocr(2);

        let pages = progressive.get_next_pages_to_process(3);
        assert_eq!(pages, vec![0, 1, 2]);

        progressive.reset();

        // Test at end of document
        progressive.set_current_page(9);
        progressive.mark_page_needs_ocr(9);
        progressive.mark_page_needs_ocr(8);
        progressive.mark_page_needs_ocr(7);

        let pages = progressive.get_next_pages_to_process(3);
        assert_eq!(pages, vec![9, 8, 7]);
    }
}
