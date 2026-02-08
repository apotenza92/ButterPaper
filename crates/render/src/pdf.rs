//! PDF document abstraction layer
//!
//! Provides a high-level interface to PDF documents using PDFium.

use crate::RenderQuality;
use pdfium_render::prelude::*;
use std::cell::OnceCell;
use std::path::Path;
use std::sync::{Mutex, OnceLock};
use std::{env, fs};

thread_local! {
    /// Thread-local Pdfium instance, leaked for 'static lifetime.
    /// Initialized once per thread on first use.
    /// Since the app runs primarily on the main thread, this effectively
    /// creates a single shared instance while avoiding Send+Sync requirements.
    static PDFIUM: OnceCell<&'static Pdfium> = const { OnceCell::new() };
}

/// Global PDFium operation lock.
///
/// PDFium page parsing / rendering is not reliably safe when the same document
/// is accessed concurrently from multiple worker threads. Serializing calls here
/// prevents sporadic heap corruption crashes during startup thumbnail + viewport rendering.
static PDFIUM_OPERATION_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

/// Minimum number of non-whitespace characters required to skip OCR
const MIN_TEXT_CHARS_THRESHOLD: usize = 50;

/// Minimum word count required to skip OCR
const MIN_WORD_COUNT_THRESHOLD: usize = 10;

/// Detect if a page needs OCR based on extracted text content
///
/// This function analyzes the extracted text from a PDF page to determine
/// if OCR is necessary. Pages with minimal or no selectable text need OCR.
///
/// # Logic
/// - Empty or whitespace-only text → needs OCR
/// - Very short text (< 50 chars) → needs OCR
/// - Few words (< 10 words) → needs OCR
/// - Otherwise → has sufficient text, skip OCR
///
/// # Arguments
/// * `text` - The extracted text from the page
///
/// # Returns
/// * `true` if the page needs OCR
/// * `false` if the page has sufficient selectable text
pub fn detect_needs_ocr(text: &str) -> bool {
    // Empty or whitespace-only pages need OCR
    if text.trim().is_empty() {
        return true;
    }

    // Count non-whitespace characters
    let char_count = text.chars().filter(|c| !c.is_whitespace()).count();
    if char_count < MIN_TEXT_CHARS_THRESHOLD {
        return true;
    }

    // Count words (sequences of alphanumeric characters)
    let word_count =
        text.split_whitespace().filter(|word| word.chars().any(|c| c.is_alphanumeric())).count();

    if word_count < MIN_WORD_COUNT_THRESHOLD {
        return true;
    }

    // Page has sufficient text, no OCR needed
    false
}

/// Errors that can occur during PDF operations
#[derive(Debug)]
pub enum PdfError {
    /// Failed to initialize PDFium library
    InitializationError(String),

    /// Failed to load PDF document
    LoadError(String),

    /// Invalid page index
    InvalidPageIndex(u16),

    /// Rendering error
    RenderError(String),
}

impl std::fmt::Display for PdfError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PdfError::InitializationError(msg) => write!(f, "PDFium initialization error: {}", msg),
            PdfError::LoadError(msg) => write!(f, "PDF load error: {}", msg),
            PdfError::InvalidPageIndex(idx) => write!(f, "Invalid page index: {}", idx),
            PdfError::RenderError(msg) => write!(f, "PDF render error: {}", msg),
        }
    }
}

impl std::error::Error for PdfError {}

/// Result type for PDF operations
pub type PdfResult<T> = Result<T, PdfError>;

/// PDF document handle
///
/// Wraps a PDFium document and provides high-level operations
/// for rendering and querying document metadata.
pub struct PdfDocument {
    /// The loaded PDF document (owns the Pdfium instance internally)
    document: pdfium_render::prelude::PdfDocument<'static>,
}

impl PdfDocument {
    fn quality_scale_factor(quality: RenderQuality) -> f32 {
        match quality {
            RenderQuality::LqThumb => 0.25,
            RenderQuality::LqScroll => 0.5,
            RenderQuality::HqFinal => 1.0,
        }
    }

    #[cfg(target_os = "windows")]
    fn platform_library_filename() -> &'static str {
        "pdfium.dll"
    }

    #[cfg(target_os = "macos")]
    fn platform_library_filename() -> &'static str {
        "libpdfium.dylib"
    }

    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    fn platform_library_filename() -> &'static str {
        "libpdfium.so"
    }

    fn push_pdfium_path_candidate(candidates: &mut Vec<std::path::PathBuf>, raw: &str) {
        let trimmed = raw.trim();

        if trimmed.is_empty() {
            return;
        }

        let path = Path::new(trimmed);
        let is_library_file = path
            .file_name()
            .map(|name| {
                name.to_string_lossy().eq_ignore_ascii_case(Self::platform_library_filename())
            })
            .unwrap_or(false);

        if is_library_file {
            candidates.push(path.to_path_buf());
        } else {
            candidates.push(path.join(Self::platform_library_filename()));
        }
    }

    fn workspace_root_from_executable() -> Option<std::path::PathBuf> {
        let exe = env::current_exe().ok()?;
        let mut dir = exe.parent()?.to_path_buf();

        loop {
            if dir.join("Cargo.toml").exists() {
                return Some(dir);
            }

            let is_target_dir = dir.file_name().map(|name| name == "target").unwrap_or(false);
            if is_target_dir {
                let parent = dir.parent()?.to_path_buf();
                if parent.join("Cargo.toml").exists() {
                    return Some(parent);
                }
            }

            if !dir.pop() {
                break;
            }
        }

        None
    }

    fn candidate_pdfium_library_paths() -> Vec<std::path::PathBuf> {
        let mut candidates = Vec::new();

        // Explicit override: absolute path to library or directory containing it.
        if let Ok(value) = env::var("BUTTERPAPER_PDFIUM_LIB") {
            Self::push_pdfium_path_candidate(&mut candidates, &value);
        }

        // Supported by pdfium-render during linking, but also useful at runtime.
        if let Ok(value) = env::var("PDFIUM_DYNAMIC_LIB_PATH") {
            Self::push_pdfium_path_candidate(&mut candidates, &value);
        }

        // macOS dynamic loader fallback path.
        if let Ok(value) = env::var("DYLD_LIBRARY_PATH") {
            for entry in env::split_paths(&value) {
                candidates.push(entry.join(Self::platform_library_filename()));
            }
        }

        // Linux dynamic loader fallback path.
        if let Ok(value) = env::var("LD_LIBRARY_PATH") {
            for entry in env::split_paths(&value) {
                candidates.push(entry.join(Self::platform_library_filename()));
            }
        }

        if let Ok(exe_path) = env::current_exe() {
            if let Some(exe_dir) = exe_path.parent() {
                candidates.push(Pdfium::pdfium_platform_library_name_at_path(exe_dir));

                #[cfg(target_os = "macos")]
                {
                    candidates.push(
                        exe_dir.join("../Frameworks").join(Self::platform_library_filename()),
                    );
                }
            }
        }

        if let Ok(cwd) = env::current_dir() {
            candidates.push(Pdfium::pdfium_platform_library_name_at_path(&cwd));
        }

        if let Some(workspace_root) = Self::workspace_root_from_executable() {
            let third_party_root = workspace_root.join("third_party").join("pdfium");
            candidates.push(third_party_root.join(Self::platform_library_filename()));

            // Common platform folders used by setup scripts.
            candidates.push(
                third_party_root
                    .join(format!("{}-{}", env::consts::OS, env::consts::ARCH))
                    .join(Self::platform_library_filename()),
            );

            candidates.push(
                third_party_root
                    .join(format!("{}-{}", env::consts::OS, env::consts::ARCH))
                    .join("lib")
                    .join(Self::platform_library_filename()),
            );

            // Last resort: discover nested library files under third_party/pdfium.
            if let Ok(entries) = fs::read_dir(&third_party_root) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if !path.is_dir() {
                        continue;
                    }

                    candidates.push(path.join(Self::platform_library_filename()));
                    candidates.push(path.join("lib").join(Self::platform_library_filename()));
                }
            }
        }

        candidates
    }

    /// Initialize PDFium library (helper function)
    ///
    /// Search order:
    /// 1. Explicit env vars (`BUTTERPAPER_PDFIUM_LIB`, `PDFIUM_DYNAMIC_LIB_PATH`)
    /// 2. Executable directory / app bundle locations
    /// 3. Workspace-local `third_party/pdfium` locations
    /// 4. Current working directory
    /// 5. System library paths
    fn init_pdfium() -> PdfResult<Pdfium> {
        for candidate in Self::candidate_pdfium_library_paths() {
            if let Ok(bindings) = Pdfium::bind_to_library(&candidate) {
                return Ok(Pdfium::new(bindings));
            }
        }

        Pdfium::bind_to_system_library().map(Pdfium::new).map_err(|system_err| {
            let hint = format!(
                "Unable to locate {}. Set BUTTERPAPER_PDFIUM_LIB to the library path \
or run scripts/setup_pdfium.sh to install a local runtime copy.",
                Self::platform_library_filename()
            );

            PdfError::InitializationError(format!("{hint}\nSystem loader error: {system_err}"))
        })
    }

    /// Get or initialize the thread-local Pdfium instance.
    ///
    /// This ensures only one Pdfium instance exists per thread,
    /// eliminating memory leaks from repeated initialization.
    /// Since the app runs on the main thread, this effectively
    /// provides a single shared instance.
    fn get_pdfium() -> PdfResult<&'static Pdfium> {
        PDFIUM.with(|cell| {
            // Check if already initialized
            if let Some(pdfium) = cell.get() {
                return Ok(*pdfium);
            }

            // Initialize Pdfium and leak it to get 'static reference
            // This is intentional - we want one instance for the app lifetime
            let pdfium = Self::init_pdfium()?;
            let pdfium_static: &'static Pdfium = Box::leak(Box::new(pdfium));

            // Store it (ignore error if another thread beat us - shouldn't happen with thread_local)
            let _ = cell.set(pdfium_static);

            Ok(pdfium_static)
        })
    }

    fn operation_lock() -> &'static Mutex<()> {
        PDFIUM_OPERATION_LOCK.get_or_init(|| Mutex::new(()))
    }

    /// Pre-initialize Pdfium library.
    ///
    /// Call this early in the application startup to ensure Pdfium is ready
    /// before any PDF operations. This moves the initialization cost away
    /// from the first PDF open.
    ///
    /// # Returns
    /// `Ok(())` if initialization succeeds, or an error if it fails.
    pub fn init_pdfium_global() -> PdfResult<()> {
        Self::get_pdfium()?;
        Ok(())
    }

    /// Load a PDF document from a file path
    ///
    /// # Arguments
    /// * `path` - Path to the PDF file
    ///
    /// # Returns
    /// A `PdfDocument` instance or an error
    pub fn open<P: AsRef<Path>>(path: P) -> PdfResult<Self> {
        // Get the shared Pdfium instance
        let pdfium = Self::get_pdfium()?;

        // Load the PDF document
        let document = pdfium
            .load_pdf_from_file(path.as_ref(), None)
            .map_err(|e| PdfError::LoadError(e.to_string()))?;

        Ok(Self { document })
    }

    /// Load a PDF document from byte data (owned)
    ///
    /// # Arguments
    /// * `data` - PDF file data as owned bytes
    ///
    /// # Returns
    /// A `PdfDocument` instance or an error
    pub fn from_bytes(data: Vec<u8>) -> PdfResult<Self> {
        // Get the shared Pdfium instance
        let pdfium = Self::get_pdfium()?;

        // Leak the data to get a 'static reference
        // Note: This is still a leak for byte data, but it's unavoidable
        // due to pdfium-render's lifetime requirements
        let data_static: &'static [u8] = Box::leak(data.into_boxed_slice());

        // Load the PDF document from bytes
        let document = pdfium
            .load_pdf_from_byte_slice(data_static, None)
            .map_err(|e| PdfError::LoadError(e.to_string()))?;

        Ok(Self { document })
    }

    /// Get the number of pages in the document
    pub fn page_count(&self) -> u16 {
        let _guard = Self::operation_lock().lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        self.document.pages().len()
    }

    /// Get a page by index (0-based)
    ///
    /// # Arguments
    /// * `index` - Zero-based page index
    ///
    /// # Returns
    /// A page reference or an error if the index is invalid
    pub fn get_page(&self, index: u16) -> PdfResult<PdfPage<'_>> {
        self.document.pages().get(index).map_err(|_| PdfError::InvalidPageIndex(index))
    }

    /// Get page dimensions in points without exposing a `PdfPage` reference.
    ///
    /// This is the preferred API for layout calculations that may overlap with
    /// background rendering work.
    pub fn page_dimensions(&self, index: u16) -> PdfResult<PageDimensions> {
        let _guard = Self::operation_lock().lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        let page =
            self.document.pages().get(index).map_err(|_| PdfError::InvalidPageIndex(index))?;

        Ok(PageDimensions { width: page.width().value, height: page.height().value })
    }

    /// Get the document's metadata
    pub fn metadata(&self) -> PdfMetadata {
        let _guard = Self::operation_lock().lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        let meta = self.document.metadata();

        PdfMetadata {
            title: meta.get(PdfDocumentMetadataTagType::Title).map(|v| v.value().to_string()),
            author: meta.get(PdfDocumentMetadataTagType::Author).map(|v| v.value().to_string()),
            subject: meta.get(PdfDocumentMetadataTagType::Subject).map(|v| v.value().to_string()),
            creator: meta.get(PdfDocumentMetadataTagType::Creator).map(|v| v.value().to_string()),
            producer: meta.get(PdfDocumentMetadataTagType::Producer).map(|v| v.value().to_string()),
        }
    }

    /// Extract all text from a specific page
    ///
    /// This extracts any selectable text embedded in the PDF page.
    /// Returns an empty string if the page has no text.
    ///
    /// # Arguments
    /// * `page_index` - Zero-based page index
    ///
    /// # Returns
    /// The extracted text or an error if the page index is invalid
    pub fn extract_page_text(&self, page_index: u16) -> PdfResult<String> {
        let _guard = Self::operation_lock().lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        let page = self
            .document
            .pages()
            .get(page_index)
            .map_err(|_| PdfError::InvalidPageIndex(page_index))?;

        // Extract text from the page
        let text = page
            .text()
            .map_err(|e| PdfError::RenderError(format!("Failed to extract text: {}", e)))?
            .all()
            .to_string();

        Ok(text)
    }

    /// Check if a page has selectable text
    ///
    /// Returns true if the page has sufficient selectable text,
    /// false if the page needs OCR.
    ///
    /// # Arguments
    /// * `page_index` - Zero-based page index
    ///
    /// # Returns
    /// True if the page has text, false otherwise
    pub fn page_has_text(&self, page_index: u16) -> PdfResult<bool> {
        let text = self.extract_page_text(page_index)?;
        Ok(!detect_needs_ocr(&text))
    }

    /// Render a page to RGBA pixel data
    ///
    /// # Arguments
    /// * `page_index` - Zero-based page index
    /// * `width` - Target width in pixels
    /// * `height` - Target height in pixels
    ///
    /// # Returns
    /// RGBA pixel data (4 bytes per pixel) or an error
    pub fn render_page_rgba(&self, page_index: u16, width: u32, height: u32) -> PdfResult<Vec<u8>> {
        self.render_page_rgba_with_quality(page_index, width, height, RenderQuality::HqFinal)
    }

    /// Render a page to RGBA pixel data using a quality profile.
    ///
    /// The provided `width` / `height` are treated as the HQ target dimensions; lower quality
    /// profiles are rendered at deterministic linear scale multipliers of that target.
    pub fn render_page_rgba_with_quality(
        &self,
        page_index: u16,
        width: u32,
        height: u32,
        quality: RenderQuality,
    ) -> PdfResult<Vec<u8>> {
        let _guard = Self::operation_lock().lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        let page = self
            .document
            .pages()
            .get(page_index)
            .map_err(|_| PdfError::InvalidPageIndex(page_index))?;

        let scale = Self::quality_scale_factor(quality);
        let target_width = ((width as f32) * scale).round().max(1.0) as u32;
        let target_height = ((height as f32) * scale).round().max(1.0) as u32;

        let config = PdfRenderConfig::new()
            .set_target_width(target_width as i32)
            .set_target_height(target_height as i32);

        let bitmap =
            page.render_with_config(&config).map_err(|e| PdfError::RenderError(e.to_string()))?;

        Ok(bitmap.as_rgba_bytes().to_vec())
    }

    /// Render a page to RGBA pixel data, scaling to fit within max dimensions
    /// while maintaining aspect ratio.
    ///
    /// # Arguments
    /// * `page_index` - Zero-based page index
    /// * `max_width` - Maximum width in pixels
    /// * `max_height` - Maximum height in pixels
    ///
    /// # Returns
    /// Tuple of (rgba_data, actual_width, actual_height) or an error
    pub fn render_page_scaled(
        &self,
        page_index: u16,
        max_width: u32,
        max_height: u32,
    ) -> PdfResult<(Vec<u8>, u32, u32)> {
        self.render_page_scaled_with_quality(
            page_index,
            max_width,
            max_height,
            RenderQuality::HqFinal,
        )
    }

    /// Render a page to RGBA pixel data, scaling to fit within max dimensions
    /// while maintaining aspect ratio and applying the given quality profile.
    pub fn render_page_scaled_with_quality(
        &self,
        page_index: u16,
        max_width: u32,
        max_height: u32,
        quality: RenderQuality,
    ) -> PdfResult<(Vec<u8>, u32, u32)> {
        let dimensions = self.page_dimensions(page_index)?;
        let page_width = dimensions.width;
        let page_height = dimensions.height;

        let scale = (max_width as f32 / page_width).min(max_height as f32 / page_height).max(0.1);

        let render_width = (page_width * scale) as u32;
        let render_height = (page_height * scale) as u32;

        let scale = Self::quality_scale_factor(quality);
        let quality_width = ((render_width as f32) * scale).round().max(1.0) as u32;
        let quality_height = ((render_height as f32) * scale).round().max(1.0) as u32;

        let rgba =
            self.render_page_rgba_with_quality(page_index, render_width, render_height, quality)?;
        Ok((rgba, quality_width, quality_height))
    }

    /// Extract text with bounding boxes from a page
    ///
    /// Returns individual text spans with their positions in page coordinates.
    /// This is used for text selection and search highlighting.
    ///
    /// # Arguments
    /// * `page_index` - Zero-based page index
    ///
    /// # Returns
    /// A vector of (text, x, y, width, height) tuples in page coordinates
    pub fn extract_text_spans(&self, page_index: u16) -> PdfResult<Vec<TextSpanInfo>> {
        let _guard = Self::operation_lock().lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        let page = self
            .document
            .pages()
            .get(page_index)
            .map_err(|_| PdfError::InvalidPageIndex(page_index))?;
        let page_height = page.height().value;

        let text_page = page
            .text()
            .map_err(|e| PdfError::RenderError(format!("Failed to get text page: {}", e)))?;

        let chars = text_page.chars();
        let mut spans = Vec::new();
        let mut current_text = String::new();
        let mut span_start_x: Option<f32> = None;
        let mut span_min_y: Option<f32> = None;
        let mut span_max_y: Option<f32> = None;
        let mut span_max_x = 0.0f32;

        // Group characters into spans (words/lines)
        for char_result in chars.iter() {
            // Get character, skip if unavailable
            let c = match char_result.unicode_char() {
                Some(ch) => ch,
                None => continue,
            };

            // Get bounds, skip if unavailable
            let loose_bounds = match char_result.loose_bounds() {
                Ok(bounds) => bounds,
                Err(_) => continue,
            };

            // Convert bounds - PDFium returns bounds with Y from bottom-left
            // Use the function accessors instead of deprecated field access
            let char_x = loose_bounds.left().value;
            let char_y = page_height - loose_bounds.top().value; // Convert to top-left origin
            let char_width = loose_bounds.right().value - loose_bounds.left().value;
            let char_height = loose_bounds.top().value - loose_bounds.bottom().value;

            // Detect word/span boundaries
            let is_whitespace = c.is_whitespace();
            let is_newline = c == '\n' || c == '\r';

            if is_whitespace || is_newline {
                // End current span if we have content
                if let (false, Some(start_x), Some(min_y), Some(max_y)) =
                    (current_text.is_empty(), span_start_x, span_min_y, span_max_y)
                {
                    spans.push(TextSpanInfo {
                        text: current_text.clone(),
                        x: start_x,
                        y: min_y,
                        width: span_max_x - start_x,
                        height: max_y - min_y,
                    });
                }
                current_text.clear();
                span_start_x = None;
                span_min_y = None;
                span_max_y = None;
                span_max_x = 0.0;
            } else {
                // Add character to current span
                current_text.push(c);

                match span_start_x {
                    None => {
                        span_start_x = Some(char_x);
                        span_min_y = Some(char_y);
                        span_max_y = Some(char_y + char_height);
                    }
                    Some(_) => {
                        span_min_y = span_min_y.map(|y| y.min(char_y));
                        span_max_y = span_max_y.map(|y| y.max(char_y + char_height));
                    }
                }
                span_max_x = span_max_x.max(char_x + char_width);
            }
        }

        // Don't forget the last span
        if let (false, Some(start_x), Some(min_y), Some(max_y)) =
            (current_text.is_empty(), span_start_x, span_min_y, span_max_y)
        {
            spans.push(TextSpanInfo {
                text: current_text,
                x: start_x,
                y: min_y,
                width: span_max_x - start_x,
                height: max_y - min_y,
            });
        }

        Ok(spans)
    }

    /// Save the PDF document to a file
    ///
    /// # Arguments
    /// * `path` - Path to save the PDF file to
    ///
    /// # Returns
    /// Ok(()) on success, or a SaveError on failure
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), SaveError> {
        let _guard = Self::operation_lock().lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        self.document.save_to_file(path.as_ref()).map_err(|e| SaveError::SaveFailed(e.to_string()))
    }

    /// Save the PDF document to bytes
    ///
    /// # Returns
    /// The PDF data as a Vec<u8> on success, or a SaveError on failure
    pub fn save_to_bytes(&self) -> Result<Vec<u8>, SaveError> {
        let _guard = Self::operation_lock().lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        self.document.save_to_bytes().map_err(|e| SaveError::SaveFailed(e.to_string()))
    }
}

/// Save error variant
#[derive(Debug)]
pub enum SaveError {
    /// Failed to save to file
    SaveFailed(String),
}

impl std::fmt::Display for SaveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SaveError::SaveFailed(msg) => write!(f, "Failed to save PDF: {}", msg),
        }
    }
}

impl std::error::Error for SaveError {}

/// PDF document metadata
#[derive(Debug, Clone, Default)]
pub struct PdfMetadata {
    pub title: Option<String>,
    pub author: Option<String>,
    pub subject: Option<String>,
    pub creator: Option<String>,
    pub producer: Option<String>,
}

/// Text span information with bounding box
#[derive(Debug, Clone)]
pub struct TextSpanInfo {
    /// The text content
    pub text: String,
    /// X coordinate of the span (left edge, page coordinates)
    pub x: f32,
    /// Y coordinate of the span (top edge, page coordinates from top-left)
    pub y: f32,
    /// Width of the span in page coordinates
    pub width: f32,
    /// Height of the span in page coordinates
    pub height: f32,
}

/// Page dimensions in points (1/72 inch)
#[derive(Debug, Clone, Copy)]
pub struct PageDimensions {
    pub width: f32,
    pub height: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pdf_error_display() {
        let err = PdfError::InvalidPageIndex(5);
        assert_eq!(err.to_string(), "Invalid page index: 5");

        let err = PdfError::LoadError("file not found".to_string());
        assert!(err.to_string().contains("file not found"));
    }

    #[test]
    fn test_metadata_default() {
        let metadata = PdfMetadata::default();
        assert!(metadata.title.is_none());
        assert!(metadata.author.is_none());
    }

    #[test]
    fn test_detect_needs_ocr_empty_text() {
        // Empty string needs OCR
        assert!(detect_needs_ocr(""));

        // Whitespace-only needs OCR
        assert!(detect_needs_ocr("   "));
        assert!(detect_needs_ocr("\n\n\t  "));
        assert!(detect_needs_ocr("     \n     "));
    }

    #[test]
    fn test_detect_needs_ocr_minimal_text() {
        // Very short text needs OCR (< 50 chars)
        assert!(detect_needs_ocr("Hello"));
        assert!(detect_needs_ocr("Page 1"));
        assert!(detect_needs_ocr("A B C D E F G"));

        // Just under threshold
        let short_text = "A".repeat(49);
        assert!(detect_needs_ocr(&short_text));
    }

    #[test]
    fn test_detect_needs_ocr_few_words() {
        // Few words need OCR (< 10 words)
        assert!(detect_needs_ocr("one two three four five"));
        assert!(detect_needs_ocr("This is a short sentence."));

        // 9 words - should need OCR
        assert!(detect_needs_ocr("one two three four five six seven eight nine"));
    }

    #[test]
    fn test_detect_needs_ocr_sufficient_text() {
        // 10+ words with 50+ chars - no OCR needed
        let text = "This is a document with sufficient text content that should not require OCR processing.";
        assert!(!detect_needs_ocr(text));

        // 100+ words - definitely no OCR needed
        let long_text = "word ".repeat(100);
        assert!(!detect_needs_ocr(&long_text));

        // Real-world example
        let document_text = "Construction plans for building 123. \
                             Floor plans indicate 3 bedrooms, 2 bathrooms. \
                             Total square footage: 2,500 sq ft. \
                             Foundation depth: 4 feet. \
                             Wall height: 9 feet.";
        assert!(!detect_needs_ocr(document_text));
    }

    #[test]
    fn test_detect_needs_ocr_edge_cases() {
        // Exactly at threshold - 50 non-whitespace chars with 10 words
        let exactly_threshold = "apple banana cherry dates elder figs grape honey iris jades";
        // This has 10 words and 50 non-whitespace characters (47 + 3 = 50)
        assert!(!detect_needs_ocr(exactly_threshold));

        // Exactly 10 words with sufficient characters
        let ten_words =
            "Documentation contains multiple words with sufficient character count here now okay";
        assert!(!detect_needs_ocr(ten_words));

        // Non-alphanumeric content doesn't count as words
        assert!(detect_needs_ocr("!!! ### $$$ %%% ^^^ &&& *** ((( ))) ___"));

        // Mixed alphanumeric and symbols
        let mixed = "Page 1 - Section A. Drawing #123. Scale: 1:100. Date: 2024-01-20.";
        assert!(!detect_needs_ocr(mixed)); // Has sufficient words and chars

        // 50 characters but only 1 word - needs OCR
        let fifty_one_word = "A".repeat(50);
        assert!(detect_needs_ocr(&fifty_one_word));

        // 9 words with 50+ chars - needs OCR (not enough words)
        let nine_words =
            "supercalifragilisticexpialidocious word three four five six seven eight nine";
        assert!(detect_needs_ocr(nine_words));
    }

    #[test]
    fn test_detect_needs_ocr_unicode() {
        // Unicode with Latin alphanumeric characters should work
        // Note: Rust's is_alphanumeric() primarily recognizes Latin/ASCII alphanumerics
        let mixed_script =
            "Engineering Document contains sufficient text content for OCR detection purposes here";
        assert!(!detect_needs_ocr(mixed_script));

        // Unicode-only text without Latin characters would need OCR by our current logic
        // This is a limitation of using is_alphanumeric() which is Latin-centric
        let unicode_only = "这是一个包含中文字符的文档";
        // This will need OCR because Chinese characters aren't detected as alphanumeric
        assert!(detect_needs_ocr(unicode_only));

        // Mixed Latin and Unicode with sufficient Latin words
        let mixed_with_latin = "Document 文档 contains sufficient Latin words to pass the threshold test successfully here";
        assert!(!detect_needs_ocr(mixed_with_latin));
    }

    #[test]
    fn test_detect_needs_ocr_scanned_page_simulation() {
        // Simulate a scanned page with just artifacts or minimal text
        // (common in construction drawings)
        assert!(detect_needs_ocr("1"));
        assert!(detect_needs_ocr("Page 1 of 50"));
        assert!(detect_needs_ocr("A1")); // Just a sheet number

        // Simulate a page with actual content
        let content_page = "FLOOR PLAN - LEVEL 1\n\
                           BEDROOM 1: 12' x 15'\n\
                           BEDROOM 2: 10' x 12'\n\
                           LIVING ROOM: 20' x 18'\n\
                           KITCHEN: 15' x 12'\n\
                           BATHROOM: 8' x 10'\n\
                           SCALE: 1/4\" = 1'-0\"\n\
                           DATE: 2024-01-15";
        assert!(!detect_needs_ocr(content_page));
    }

    #[test]
    fn test_executable_path_detection() {
        // Verify that we can get the executable's directory
        // This is used to find libpdfium.dylib in app bundles
        let exe_dir =
            std::env::current_exe().ok().and_then(|p| p.parent().map(|p| p.to_path_buf()));

        // Should always be able to get the executable directory
        assert!(exe_dir.is_some(), "Failed to get executable directory");

        // The directory should exist
        let dir = exe_dir.unwrap();
        assert!(dir.exists(), "Executable directory does not exist");

        // The directory should be absolute
        assert!(dir.is_absolute(), "Executable directory should be absolute");
    }

    #[test]
    fn test_pdfium_library_name_generation() {
        // Test that the library name is generated correctly for the platform
        let exe_dir =
            std::env::current_exe().ok().and_then(|p| p.parent().map(|p| p.to_path_buf()));

        if let Some(dir) = exe_dir {
            let lib_path = Pdfium::pdfium_platform_library_name_at_path(&dir);
            let lib_name = lib_path.to_string_lossy();

            // On macOS, should end with .dylib
            #[cfg(target_os = "macos")]
            assert!(
                lib_name.ends_with(".dylib"),
                "Expected .dylib extension on macOS, got: {}",
                lib_name
            );

            // On Linux, should end with .so
            #[cfg(target_os = "linux")]
            assert!(
                lib_name.ends_with(".so"),
                "Expected .so extension on Linux, got: {}",
                lib_name
            );

            // On Windows, should end with .dll
            #[cfg(target_os = "windows")]
            assert!(
                lib_name.ends_with(".dll"),
                "Expected .dll extension on Windows, got: {}",
                lib_name
            );

            // Should contain "pdfium"
            assert!(
                lib_name.to_lowercase().contains("pdfium"),
                "Library name should contain 'pdfium', got: {}",
                lib_name
            );
        }
    }

    #[test]
    fn test_save_error_display() {
        let err = SaveError::SaveFailed("permission denied".to_string());
        let display = err.to_string();
        assert!(display.contains("Failed to save PDF"));
        assert!(display.contains("permission denied"));
    }

    #[test]
    fn test_save_error_debug() {
        let err = SaveError::SaveFailed("test error".to_string());
        let debug_str = format!("{:?}", err);
        assert!(debug_str.contains("SaveFailed"));
        assert!(debug_str.contains("test error"));
    }

    #[test]
    fn test_push_pdfium_path_candidate_from_directory() {
        let mut candidates = Vec::new();
        PdfDocument::push_pdfium_path_candidate(&mut candidates, "/tmp/pdfium");
        assert_eq!(candidates.len(), 1);
        assert!(candidates[0].to_string_lossy().contains(PdfDocument::platform_library_filename()));
    }

    #[test]
    fn test_push_pdfium_path_candidate_from_file() {
        let mut candidates = Vec::new();

        #[cfg(target_os = "windows")]
        let path = r"C:\pdfium\pdfium.dll";
        #[cfg(not(target_os = "windows"))]
        let path = "/tmp/libpdfium.dylib";

        PdfDocument::push_pdfium_path_candidate(&mut candidates, path);
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0], Path::new(path));
    }
}
