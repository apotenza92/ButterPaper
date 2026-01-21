//! Fast document loader with metadata-only initialization
//!
//! Provides fast file opening by loading only metadata initially.
//! Page rendering is deferred to the render pipeline.

use crate::document::{DocumentError, DocumentId, DocumentManager, DocumentMetadata, DocumentResult};
use pdf_editor_render::PdfDocument;
use std::path::Path;

/// Document loader for fast file opening
///
/// Loads only document metadata (title, author, page count, etc.)
/// during the initial file open. Page content rendering is deferred.
pub struct DocumentLoader {
    /// Document manager for tracking open documents
    manager: DocumentManager,
}

impl DocumentLoader {
    /// Create a new document loader
    pub fn new(manager: DocumentManager) -> Self {
        Self { manager }
    }

    /// Fast file open - loads metadata only
    ///
    /// This method opens a PDF file and extracts only the metadata
    /// (page count, title, author, etc.) without rendering any pages.
    /// This enables near-instant file opening even for large documents.
    ///
    /// Returns the document ID for the newly opened document.
    pub fn open_file<P: AsRef<Path>>(&self, path: P) -> DocumentResult<DocumentId> {
        let path = path.as_ref();

        // Validate file exists
        if !path.exists() {
            return Err(DocumentError::LoadError(format!(
                "File not found: {}",
                path.display()
            )));
        }

        // Get file size
        let file_size = std::fs::metadata(path)
            .map(|m| m.len())
            .unwrap_or(0);

        // Load metadata using pdf-editor-render crate
        // This is a fast operation that doesn't render any pages
        let metadata = self.load_metadata(path, file_size)?;

        // Register with document manager
        let doc_id = self.manager.register_document(metadata);

        Ok(doc_id)
    }

    /// Load document metadata from PDF file
    ///
    /// This is the fast path - only metadata extraction, no rendering.
    fn load_metadata(&self, path: &Path, file_size: u64) -> DocumentResult<DocumentMetadata> {
        // Open PDF document using render crate
        let pdf_doc = PdfDocument::open(path)
            .map_err(|e| DocumentError::LoadError(format!("Failed to open PDF: {}", e)))?;

        // Extract metadata (fast operation, no rendering)
        let pdf_metadata = pdf_doc.metadata();
        let page_count = pdf_doc.page_count();

        // Build DocumentMetadata
        let mut metadata = DocumentMetadata {
            title: pdf_metadata.title,
            author: pdf_metadata.author,
            subject: pdf_metadata.subject,
            creator: pdf_metadata.creator,
            producer: pdf_metadata.producer,
            page_count,
            file_path: path.to_path_buf(),
            file_size,
            scale_systems: Vec::new(),
            default_scales: std::collections::HashMap::new(),
        };

        // Try to load persisted metadata (scale systems, etc.)
        if let Ok(Some(persisted)) = crate::persistence::load_metadata(path) {
            metadata.scale_systems = persisted.scale_systems;
            metadata.default_scales = persisted.default_scales;
        }

        // Note: pdf_doc is dropped here, closing the file
        // This keeps memory usage minimal during the metadata-only phase

        Ok(metadata)
    }

    /// Get the document manager
    pub fn manager(&self) -> &DocumentManager {
        &self.manager
    }
}

/// Configuration for document loading
#[derive(Debug, Clone)]
pub struct LoaderConfig {
    /// Maximum number of documents to keep open simultaneously
    pub max_open_documents: usize,

    /// Whether to automatically close oldest document when limit is reached
    pub auto_close_old_documents: bool,
}

impl Default for LoaderConfig {
    fn default() -> Self {
        Self {
            max_open_documents: 10,
            auto_close_old_documents: true,
        }
    }
}

impl LoaderConfig {
    /// Create a new loader configuration
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the maximum number of open documents
    pub fn with_max_open_documents(mut self, max: usize) -> Self {
        self.max_open_documents = max;
        self
    }

    /// Set whether to automatically close old documents
    pub fn with_auto_close(mut self, auto_close: bool) -> Self {
        self.auto_close_old_documents = auto_close;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_loader_creation() {
        let manager = DocumentManager::new();
        let loader = DocumentLoader::new(manager);
        assert_eq!(loader.manager().document_count(), 0);
    }

    #[test]
    fn test_open_file_not_found() {
        let manager = DocumentManager::new();
        let loader = DocumentLoader::new(manager);

        let result = loader.open_file("/nonexistent/file.pdf");
        assert!(result.is_err());

        if let Err(DocumentError::LoadError(msg)) = result {
            assert!(msg.contains("File not found"));
        } else {
            panic!("Expected LoadError");
        }
    }

    #[test]
    fn test_loader_config_default() {
        let config = LoaderConfig::default();
        assert_eq!(config.max_open_documents, 10);
        assert!(config.auto_close_old_documents);
    }

    #[test]
    fn test_loader_config_builder() {
        let config = LoaderConfig::new()
            .with_max_open_documents(5)
            .with_auto_close(false);

        assert_eq!(config.max_open_documents, 5);
        assert!(!config.auto_close_old_documents);
    }
}
