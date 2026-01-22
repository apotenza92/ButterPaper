use std::sync::Mutex;

/// Application state for the Tauri app.
///
/// Note: PdfDocument is not stored here because pdfium's internal types
/// are not Send+Sync safe. Instead, we store the file path and re-open
/// the document when needed. PDFium handles internal caching efficiently.
pub struct AppState {
    pub file_path: Mutex<Option<String>>,
    pub page_count: Mutex<u32>,          // Cached page count
    pub document_title: Mutex<Option<String>>, // Cached title
    pub current_page: Mutex<u32>,        // 0-indexed
    pub zoom_percent: Mutex<u32>,        // 100 = 100%
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            file_path: Mutex::new(None),
            page_count: Mutex::new(0),
            document_title: Mutex::new(None),
            current_page: Mutex::new(0),
            zoom_percent: Mutex::new(100),
        }
    }
}
