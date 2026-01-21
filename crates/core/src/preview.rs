//! First-page immediate preview rendering
//!
//! Provides fast first-page preview rendering immediately after file open.
//! Preview tiles are rendered at lower quality for quick display.

use crate::document::{Document, DocumentError, DocumentId, DocumentResult};
use pdf_editor_render::{PdfDocument, RenderedTile, TileProfile, TileRenderer};
use std::sync::{Arc, Mutex};

/// Preview render result
#[derive(Debug, Clone)]
pub struct PreviewResult {
    /// Document ID
    pub document_id: DocumentId,

    /// First page preview tiles
    pub tiles: Vec<RenderedTile>,

    /// Page width in pixels at 100% zoom
    pub page_width: u32,

    /// Page height in pixels at 100% zoom
    pub page_height: u32,

    /// Zoom level used for rendering
    pub zoom_level: u32,
}

/// First-page preview renderer
///
/// Renders the first page of a document immediately after opening
/// using the preview profile for fast display.
pub struct PreviewRenderer {
    /// Tile renderer for rendering page tiles
    tile_renderer: TileRenderer,

    /// Default zoom level for preview (100% = actual size)
    default_zoom: u32,
}

impl PreviewRenderer {
    /// Create a new preview renderer
    pub fn new() -> Self {
        Self {
            tile_renderer: TileRenderer::new(),
            default_zoom: 100,
        }
    }

    /// Create a new preview renderer with custom zoom level
    pub fn with_zoom(zoom_level: u32) -> Self {
        Self {
            tile_renderer: TileRenderer::new(),
            default_zoom: zoom_level,
        }
    }

    /// Render first-page preview for a document
    ///
    /// This method renders all tiles for the first page using the preview profile
    /// for fast display. The tiles are rendered at the default zoom level.
    ///
    /// # Arguments
    /// * `document` - The document to render
    ///
    /// # Returns
    /// A `PreviewResult` with the rendered tiles and page dimensions
    pub fn render_first_page(&self, document: &Document) -> DocumentResult<PreviewResult> {
        let file_path = &document.metadata().file_path;

        // Open PDF document for rendering
        let pdf_doc = PdfDocument::open(file_path)
            .map_err(|e| DocumentError::LoadError(format!("Failed to open PDF: {}", e)))?;

        // Ensure document has at least one page
        if pdf_doc.page_count() == 0 {
            return Err(DocumentError::LoadError("Document has no pages".to_string()));
        }

        // Get first page dimensions
        let page = pdf_doc.get_page(0)
            .map_err(|e| DocumentError::LoadError(format!("Failed to get first page: {}", e)))?;
        let page_width = page.width().value as u32;
        let page_height = page.height().value as u32;

        // Render all tiles for first page using preview profile
        let tiles = self.tile_renderer
            .render_page_tiles(&pdf_doc, 0, self.default_zoom, TileProfile::Preview)
            .map_err(|e| DocumentError::LoadError(format!("Failed to render tiles: {}", e)))?;

        Ok(PreviewResult {
            document_id: document.id(),
            tiles,
            page_width,
            page_height,
            zoom_level: self.default_zoom,
        })
    }

    /// Render first-page preview at a specific zoom level
    ///
    /// This is useful when you want to render the preview at a different zoom level
    /// than the default (e.g., for fitting the page to viewport).
    ///
    /// # Arguments
    /// * `document` - The document to render
    /// * `zoom_level` - The zoom level to use (100 = 100%)
    ///
    /// # Returns
    /// A `PreviewResult` with the rendered tiles and page dimensions
    pub fn render_first_page_at_zoom(
        &self,
        document: &Document,
        zoom_level: u32,
    ) -> DocumentResult<PreviewResult> {
        let file_path = &document.metadata().file_path;

        // Open PDF document for rendering
        let pdf_doc = PdfDocument::open(file_path)
            .map_err(|e| DocumentError::LoadError(format!("Failed to open PDF: {}", e)))?;

        // Ensure document has at least one page
        if pdf_doc.page_count() == 0 {
            return Err(DocumentError::LoadError("Document has no pages".to_string()));
        }

        // Get first page dimensions
        let page = pdf_doc.get_page(0)
            .map_err(|e| DocumentError::LoadError(format!("Failed to get first page: {}", e)))?;
        let page_width = page.width().value as u32;
        let page_height = page.height().value as u32;

        // Render all tiles for first page using preview profile
        let tiles = self.tile_renderer
            .render_page_tiles(&pdf_doc, 0, zoom_level, TileProfile::Preview)
            .map_err(|e| DocumentError::LoadError(format!("Failed to render tiles: {}", e)))?;

        Ok(PreviewResult {
            document_id: document.id(),
            tiles,
            page_width,
            page_height,
            zoom_level,
        })
    }

    /// Calculate the zoom level needed to fit a page to a viewport
    ///
    /// Returns the zoom level (as a percentage) that will fit the page
    /// within the given viewport dimensions.
    ///
    /// # Arguments
    /// * `page_width` - Page width in pixels at 100% zoom
    /// * `page_height` - Page height in pixels at 100% zoom
    /// * `viewport_width` - Viewport width in pixels
    /// * `viewport_height` - Viewport height in pixels
    ///
    /// # Returns
    /// Zoom level as a percentage (e.g., 100 = 100%)
    pub fn calculate_fit_zoom(
        page_width: u32,
        page_height: u32,
        viewport_width: u32,
        viewport_height: u32,
    ) -> u32 {
        // Calculate zoom to fit width and height
        let zoom_width = (viewport_width as f32 / page_width as f32) * 100.0;
        let zoom_height = (viewport_height as f32 / page_height as f32) * 100.0;

        // Use the smaller zoom to ensure entire page fits
        zoom_width.min(zoom_height) as u32
    }

    /// Get the tile renderer
    pub fn tile_renderer(&self) -> &TileRenderer {
        &self.tile_renderer
    }

    /// Get the default zoom level
    pub fn default_zoom(&self) -> u32 {
        self.default_zoom
    }
}

impl Default for PreviewRenderer {
    fn default() -> Self {
        Self::new()
    }
}

/// Asynchronous preview renderer for non-blocking preview generation
///
/// This renderer wraps the synchronous PreviewRenderer and provides
/// methods for rendering previews on a background thread without blocking
/// the UI thread.
pub struct AsyncPreviewRenderer {
    /// Synchronous preview renderer
    renderer: Arc<PreviewRenderer>,
}

impl AsyncPreviewRenderer {
    /// Create a new async preview renderer
    pub fn new(renderer: PreviewRenderer) -> Self {
        Self {
            renderer: Arc::new(renderer),
        }
    }

    /// Render first-page preview asynchronously
    ///
    /// This method spawns a background thread to render the preview
    /// and returns a handle to wait for the result.
    ///
    /// # Arguments
    /// * `document` - The document to render (cloned for thread safety)
    ///
    /// # Returns
    /// A `PreviewHandle` that can be used to wait for the result
    pub fn render_first_page_async(&self, document: Document) -> PreviewHandle {
        let renderer = Arc::clone(&self.renderer);
        let result = Arc::new(Mutex::new(None));
        let result_clone = Arc::clone(&result);

        let handle = std::thread::spawn(move || {
            let preview_result = renderer.render_first_page(&document);
            *result_clone.lock().unwrap() = Some(preview_result);
        });

        PreviewHandle {
            thread: Some(handle),
            result,
        }
    }
}

/// Handle to an asynchronous preview rendering operation
pub struct PreviewHandle {
    thread: Option<std::thread::JoinHandle<()>>,
    result: Arc<Mutex<Option<DocumentResult<PreviewResult>>>>,
}

impl PreviewHandle {
    /// Check if the preview rendering is complete
    pub fn is_complete(&self) -> bool {
        self.result.lock().unwrap().is_some()
    }

    /// Try to get the result without blocking
    ///
    /// Returns `Some(result)` if complete, `None` if still rendering.
    pub fn try_get(&mut self) -> Option<DocumentResult<PreviewResult>> {
        self.result.lock().unwrap().take()
    }

    /// Wait for the preview rendering to complete and return the result
    ///
    /// This will block the calling thread until rendering is complete.
    pub fn wait(mut self) -> DocumentResult<PreviewResult> {
        if let Some(handle) = self.thread.take() {
            handle.join().expect("Preview rendering thread panicked");
        }

        self.result.lock().unwrap().take().expect("No result available")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::DocumentMetadata;
    use std::path::PathBuf;

    fn test_document() -> Document {
        let metadata = DocumentMetadata {
            title: Some("Test Document".to_string()),
            author: Some("Test Author".to_string()),
            subject: None,
            creator: None,
            producer: None,
            page_count: 10,
            file_path: PathBuf::from("/nonexistent/test.pdf"),
            file_size: 1024,
        };

        Document::new(1, metadata)
    }

    #[test]
    fn test_preview_renderer_creation() {
        let renderer = PreviewRenderer::new();
        assert_eq!(renderer.default_zoom(), 100);
        assert_eq!(renderer.tile_renderer().tile_size(), 256);
    }

    #[test]
    fn test_preview_renderer_with_zoom() {
        let renderer = PreviewRenderer::with_zoom(150);
        assert_eq!(renderer.default_zoom(), 150);
    }

    #[test]
    fn test_render_first_page_nonexistent_file() {
        let renderer = PreviewRenderer::new();
        let document = test_document();

        let result = renderer.render_first_page(&document);
        assert!(result.is_err());
    }

    #[test]
    fn test_calculate_fit_zoom() {
        // Page: 612x792 (US Letter)
        // Viewport: 800x600

        // Should fit width: 800/612 * 100 = 130.7%
        // Should fit height: 600/792 * 100 = 75.7%
        // Use smaller (height) = 75%
        let zoom = PreviewRenderer::calculate_fit_zoom(612, 792, 800, 600);
        assert_eq!(zoom, 75);
    }

    #[test]
    fn test_calculate_fit_zoom_portrait() {
        // Page: 800x1200 (portrait)
        // Viewport: 600x800

        // Should fit width: 600/800 * 100 = 75%
        // Should fit height: 800/1200 * 100 = 66.6%
        // Use smaller (height) = 66%
        let zoom = PreviewRenderer::calculate_fit_zoom(800, 1200, 600, 800);
        assert_eq!(zoom, 66);
    }

    #[test]
    fn test_calculate_fit_zoom_exact_fit() {
        // Page and viewport same size
        let zoom = PreviewRenderer::calculate_fit_zoom(800, 600, 800, 600);
        assert_eq!(zoom, 100);
    }

    #[test]
    fn test_async_preview_renderer_creation() {
        let renderer = PreviewRenderer::new();
        let async_renderer = AsyncPreviewRenderer::new(renderer);

        // Just verify it compiles and constructs correctly
        let _ = async_renderer;
    }

    #[test]
    fn test_preview_handle_is_complete() {
        // Create a handle with a completed result
        let result = Arc::new(Mutex::new(Some(Err(DocumentError::LoadError(
            "Test error".to_string(),
        )))));

        let handle = PreviewHandle {
            thread: None,
            result,
        };

        assert!(handle.is_complete());
    }

    #[test]
    fn test_preview_handle_try_get() {
        // Create a handle with a completed result
        let result = Arc::new(Mutex::new(Some(Err(DocumentError::LoadError(
            "Test error".to_string(),
        )))));

        let mut handle = PreviewHandle {
            thread: None,
            result,
        };

        let result = handle.try_get();
        assert!(result.is_some());
        assert!(result.unwrap().is_err());
    }
}
