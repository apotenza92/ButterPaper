//! OCR subsystem for extracting text from PDF pages
//!
//! Provides integration with Tesseract OCR for extracting text from
//! pages that don't have selectable text. OCR runs progressively and
//! non-blocking in the background.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// OCR engine configuration
#[derive(Debug, Clone)]
pub struct OcrConfig {
    /// Path to Tesseract data directory (tessdata)
    pub tessdata_path: Option<PathBuf>,

    /// Language to use for OCR (e.g., "eng" for English)
    pub language: String,

    /// OCR engine mode (0=Original, 1=Neural nets LSTM, 2=Legacy+LSTM, 3=Default)
    pub engine_mode: i32,

    /// Page segmentation mode (3=Auto, 6=Single uniform block, etc.)
    pub page_segmentation_mode: i32,

    /// Whether to run OCR in progressive mode (current page first)
    pub progressive: bool,

    /// Maximum number of concurrent OCR operations
    pub max_concurrent: usize,
}

impl Default for OcrConfig {
    fn default() -> Self {
        Self {
            tessdata_path: None, // Will use system default
            language: "eng".to_string(),
            engine_mode: 3, // Default mode
            page_segmentation_mode: 3, // Auto
            progressive: true,
            max_concurrent: 2,
        }
    }
}

impl OcrConfig {
    /// Create a new OCR configuration with defaults
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the tessdata path
    pub fn with_tessdata_path(mut self, path: PathBuf) -> Self {
        self.tessdata_path = Some(path);
        self
    }

    /// Set the OCR language
    pub fn with_language(mut self, language: impl Into<String>) -> Self {
        self.language = language.into();
        self
    }

    /// Set the engine mode
    pub fn with_engine_mode(mut self, mode: i32) -> Self {
        self.engine_mode = mode;
        self
    }

    /// Set the page segmentation mode
    pub fn with_page_segmentation_mode(mut self, mode: i32) -> Self {
        self.page_segmentation_mode = mode;
        self
    }

    /// Enable or disable progressive OCR
    pub fn with_progressive(mut self, progressive: bool) -> Self {
        self.progressive = progressive;
        self
    }

    /// Set maximum concurrent OCR operations
    pub fn with_max_concurrent(mut self, max: usize) -> Self {
        self.max_concurrent = max;
        self
    }
}

/// OCR result for a page
#[derive(Debug, Clone)]
pub struct OcrResult {
    /// Page index this result is for
    pub page_index: u16,

    /// Extracted text content
    pub text: String,

    /// Text blocks with positions (for invisible text layer)
    pub text_blocks: Vec<TextBlock>,

    /// Confidence score (0.0 to 1.0)
    pub confidence: f32,

    /// Whether the page had pre-existing selectable text
    pub had_existing_text: bool,
}

impl OcrResult {
    /// Create a new OCR result
    pub fn new(page_index: u16, text: String, text_blocks: Vec<TextBlock>, confidence: f32) -> Self {
        Self {
            page_index,
            text,
            text_blocks,
            confidence,
            had_existing_text: false,
        }
    }

    /// Create an OCR result for a page that already had text
    pub fn existing_text(page_index: u16, text: String) -> Self {
        Self {
            page_index,
            text,
            text_blocks: Vec::new(),
            confidence: 1.0,
            had_existing_text: true,
        }
    }

    /// Check if this result is reliable (confidence above threshold)
    pub fn is_reliable(&self) -> bool {
        self.confidence >= 0.7 || self.had_existing_text
    }
}

/// A text block with position information
#[derive(Debug, Clone)]
pub struct TextBlock {
    /// Text content of this block
    pub text: String,

    /// Bounding box in page coordinates (x, y, width, height)
    pub bbox: (f32, f32, f32, f32),

    /// Confidence score for this block (0.0 to 1.0)
    pub confidence: f32,

    /// Font size estimate (in points)
    pub font_size: f32,
}

impl TextBlock {
    /// Create a new text block
    pub fn new(text: String, bbox: (f32, f32, f32, f32), confidence: f32, font_size: f32) -> Self {
        Self {
            text,
            bbox,
            confidence,
            font_size,
        }
    }
}

/// OCR engine state
#[derive(Debug)]
struct OcrEngineState {
    /// Configuration
    config: OcrConfig,

    /// Whether the engine is initialized
    initialized: bool,

    /// Number of active OCR operations
    active_operations: usize,
}

/// Local OCR engine using Tesseract
///
/// This engine runs OCR operations locally without any network dependencies.
/// It's designed to be thread-safe and non-blocking.
pub struct OcrEngine {
    state: Arc<Mutex<OcrEngineState>>,
}

impl OcrEngine {
    /// Create a new OCR engine with the given configuration
    pub fn new(config: OcrConfig) -> Result<Self, OcrError> {
        let state = OcrEngineState {
            config,
            initialized: false,
            active_operations: 0,
        };

        Ok(Self {
            state: Arc::new(Mutex::new(state)),
        })
    }

    /// Initialize the OCR engine
    ///
    /// This should be called before performing any OCR operations.
    /// It's separated from construction to allow lazy initialization.
    pub fn initialize(&self) -> Result<(), OcrError> {
        let mut state = self.state.lock().unwrap();

        if state.initialized {
            return Ok(());
        }

        // TODO: Initialize Tesseract library
        // This will be implemented when we add the tesseract-rs dependency

        state.initialized = true;
        Ok(())
    }

    /// Check if a page needs OCR (doesn't have selectable text)
    ///
    /// This should be called before scheduling OCR to avoid unnecessary work.
    /// Pages with minimal or no selectable text are considered to need OCR.
    ///
    /// The actual detection logic is implemented in the render crate's
    /// `detect_needs_ocr` function, which checks for sufficient text content.
    ///
    /// # Arguments
    /// * `extracted_text` - Text already extracted from the page (if any)
    ///
    /// # Returns
    /// * `true` if the page needs OCR (no or minimal text)
    /// * `false` if the page has sufficient selectable text
    pub fn needs_ocr(&self, extracted_text: &str) -> bool {
        // Delegate to the render crate's detection function
        pdf_editor_render::detect_needs_ocr(extracted_text)
    }

    /// Perform OCR on a page image
    ///
    /// This is the main OCR operation. It takes a rendered page image
    /// and extracts text with position information.
    ///
    /// # Arguments
    /// * `page_index` - The index of the page being processed
    /// * `image_data` - Raw image data (RGBA format)
    /// * `width` - Image width in pixels
    /// * `height` - Image height in pixels
    pub fn process_page(
        &self,
        page_index: u16,
        _image_data: &[u8],
        _width: u32,
        _height: u32,
    ) -> Result<OcrResult, OcrError> {
        let mut state = self.state.lock().unwrap();

        if !state.initialized {
            return Err(OcrError::NotInitialized);
        }

        if state.active_operations >= state.config.max_concurrent {
            return Err(OcrError::TooManyOperations);
        }

        state.active_operations += 1;
        drop(state); // Release lock during OCR operation

        // TODO: Perform actual OCR using Tesseract
        // For now, return a placeholder result
        let result = OcrResult::new(
            page_index,
            String::new(),
            Vec::new(),
            0.0,
        );

        // Decrement active operations
        let mut state = self.state.lock().unwrap();
        state.active_operations -= 1;

        Ok(result)
    }

    /// Extract existing text from a PDF page
    ///
    /// This extracts any pre-existing selectable text from the PDF
    /// without running OCR. Returns None if the page has no text.
    pub fn extract_existing_text(
        &self,
        page_index: u16,
        _page_data: &[u8],
    ) -> Result<Option<OcrResult>, OcrError> {
        // TODO: Implement PDF text extraction
        // For now, return None (no existing text)
        let _ = page_index;
        Ok(None)
    }

    /// Get the current configuration
    pub fn config(&self) -> OcrConfig {
        let state = self.state.lock().unwrap();
        state.config.clone()
    }

    /// Check if the engine is initialized
    pub fn is_initialized(&self) -> bool {
        let state = self.state.lock().unwrap();
        state.initialized
    }

    /// Get the number of active OCR operations
    pub fn active_operations(&self) -> usize {
        let state = self.state.lock().unwrap();
        state.active_operations
    }
}

/// OCR error types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OcrError {
    /// Engine not initialized
    NotInitialized,

    /// Too many concurrent operations
    TooManyOperations,

    /// Tesseract initialization failed
    InitializationFailed(String),

    /// OCR processing failed
    ProcessingFailed(String),

    /// Invalid image data
    InvalidImage(String),

    /// Language data not found
    LanguageNotFound(String),
}

impl std::fmt::Display for OcrError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OcrError::NotInitialized => write!(f, "OCR engine not initialized"),
            OcrError::TooManyOperations => write!(f, "Too many concurrent OCR operations"),
            OcrError::InitializationFailed(msg) => write!(f, "OCR initialization failed: {}", msg),
            OcrError::ProcessingFailed(msg) => write!(f, "OCR processing failed: {}", msg),
            OcrError::InvalidImage(msg) => write!(f, "Invalid image data: {}", msg),
            OcrError::LanguageNotFound(lang) => write!(f, "OCR language data not found: {}", lang),
        }
    }
}

impl std::error::Error for OcrError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ocr_config_default() {
        let config = OcrConfig::default();
        assert_eq!(config.language, "eng");
        assert_eq!(config.engine_mode, 3);
        assert_eq!(config.page_segmentation_mode, 3);
        assert!(config.progressive);
        assert_eq!(config.max_concurrent, 2);
        assert!(config.tessdata_path.is_none());
    }

    #[test]
    fn test_ocr_config_builder() {
        let config = OcrConfig::new()
            .with_language("fra")
            .with_engine_mode(1)
            .with_page_segmentation_mode(6)
            .with_progressive(false)
            .with_max_concurrent(4)
            .with_tessdata_path(PathBuf::from("/usr/share/tessdata"));

        assert_eq!(config.language, "fra");
        assert_eq!(config.engine_mode, 1);
        assert_eq!(config.page_segmentation_mode, 6);
        assert!(!config.progressive);
        assert_eq!(config.max_concurrent, 4);
        assert_eq!(config.tessdata_path, Some(PathBuf::from("/usr/share/tessdata")));
    }

    #[test]
    fn test_ocr_result_creation() {
        let text_blocks = vec![
            TextBlock::new("Hello".to_string(), (0.0, 0.0, 100.0, 20.0), 0.95, 12.0),
        ];

        let result = OcrResult::new(0, "Hello".to_string(), text_blocks, 0.95);
        assert_eq!(result.page_index, 0);
        assert_eq!(result.text, "Hello");
        assert_eq!(result.text_blocks.len(), 1);
        assert_eq!(result.confidence, 0.95);
        assert!(!result.had_existing_text);
    }

    #[test]
    fn test_ocr_result_existing_text() {
        let result = OcrResult::existing_text(1, "Pre-existing text".to_string());
        assert_eq!(result.page_index, 1);
        assert_eq!(result.text, "Pre-existing text");
        assert_eq!(result.text_blocks.len(), 0);
        assert_eq!(result.confidence, 1.0);
        assert!(result.had_existing_text);
    }

    #[test]
    fn test_ocr_result_is_reliable() {
        let reliable = OcrResult::new(0, "test".to_string(), Vec::new(), 0.8);
        assert!(reliable.is_reliable());

        let unreliable = OcrResult::new(0, "test".to_string(), Vec::new(), 0.5);
        assert!(!unreliable.is_reliable());

        let existing = OcrResult::existing_text(0, "test".to_string());
        assert!(existing.is_reliable());
    }

    #[test]
    fn test_text_block_creation() {
        let block = TextBlock::new(
            "Test text".to_string(),
            (10.0, 20.0, 100.0, 15.0),
            0.9,
            14.0,
        );

        assert_eq!(block.text, "Test text");
        assert_eq!(block.bbox, (10.0, 20.0, 100.0, 15.0));
        assert_eq!(block.confidence, 0.9);
        assert_eq!(block.font_size, 14.0);
    }

    #[test]
    fn test_ocr_engine_creation() {
        let config = OcrConfig::default();
        let engine = OcrEngine::new(config);
        assert!(engine.is_ok());

        let engine = engine.unwrap();
        assert!(!engine.is_initialized());
        assert_eq!(engine.active_operations(), 0);
    }

    #[test]
    fn test_ocr_engine_initialization() {
        let config = OcrConfig::default();
        let engine = OcrEngine::new(config).unwrap();

        assert!(!engine.is_initialized());

        let result = engine.initialize();
        assert!(result.is_ok());
        assert!(engine.is_initialized());

        // Should be idempotent
        let result = engine.initialize();
        assert!(result.is_ok());
        assert!(engine.is_initialized());
    }

    #[test]
    fn test_ocr_engine_process_page_not_initialized() {
        let config = OcrConfig::default();
        let engine = OcrEngine::new(config).unwrap();

        let image_data = vec![0u8; 1000];
        let result = engine.process_page(0, &image_data, 100, 100);

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), OcrError::NotInitialized);
    }

    #[test]
    fn test_ocr_engine_extract_existing_text() {
        let config = OcrConfig::default();
        let engine = OcrEngine::new(config).unwrap();

        let page_data = vec![0u8; 1000];
        let result = engine.extract_existing_text(0, &page_data);

        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn test_ocr_error_display() {
        let error = OcrError::NotInitialized;
        assert_eq!(error.to_string(), "OCR engine not initialized");

        let error = OcrError::TooManyOperations;
        assert_eq!(error.to_string(), "Too many concurrent OCR operations");

        let error = OcrError::InitializationFailed("test".to_string());
        assert_eq!(error.to_string(), "OCR initialization failed: test");

        let error = OcrError::ProcessingFailed("test".to_string());
        assert_eq!(error.to_string(), "OCR processing failed: test");

        let error = OcrError::InvalidImage("test".to_string());
        assert_eq!(error.to_string(), "Invalid image data: test");

        let error = OcrError::LanguageNotFound("eng".to_string());
        assert_eq!(error.to_string(), "OCR language data not found: eng");
    }

    #[test]
    fn test_ocr_engine_needs_ocr() {
        use pdf_editor_render::detect_needs_ocr;

        let config = OcrConfig::default();
        let engine = OcrEngine::new(config).unwrap();

        // Test various scenarios
        assert!(engine.needs_ocr(""));
        assert!(engine.needs_ocr("Page 1"));

        // Sufficient text: 10+ words and 50+ chars
        let sufficient_text = "This is a document with sufficient text content that should not require OCR processing.";
        assert!(!engine.needs_ocr(sufficient_text));

        // Test the detection function directly
        assert!(detect_needs_ocr(""));
        assert!(detect_needs_ocr("Page 1"));
        assert!(!detect_needs_ocr(sufficient_text));
    }
}
