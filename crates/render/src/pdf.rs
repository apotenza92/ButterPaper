//! PDF document abstraction layer
//!
//! Provides a high-level interface to PDF documents using PDFium.

use pdfium_render::prelude::*;
use std::path::Path;

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
    /// Initialize PDFium library (helper function)
    fn init_pdfium() -> PdfResult<Pdfium> {
        Ok(Pdfium::new(
            Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path("./"))
                .or_else(|_| Pdfium::bind_to_system_library())
                .map_err(|e| PdfError::InitializationError(e.to_string()))?,
        ))
    }

    /// Load a PDF document from a file path
    ///
    /// # Arguments
    /// * `path` - Path to the PDF file
    ///
    /// # Returns
    /// A `PdfDocument` instance or an error
    pub fn open<P: AsRef<Path>>(path: P) -> PdfResult<Self> {
        // Initialize PDFium library
        let pdfium = Box::leak(Box::new(Self::init_pdfium()?));

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
        // Initialize PDFium library
        let pdfium = Box::leak(Box::new(Self::init_pdfium()?));

        // Leak the data to get a 'static reference
        let data_static: &'static [u8] = Box::leak(data.into_boxed_slice());

        // Load the PDF document from bytes
        let document = pdfium
            .load_pdf_from_byte_slice(data_static, None)
            .map_err(|e| PdfError::LoadError(e.to_string()))?;

        Ok(Self { document })
    }

    /// Get the number of pages in the document
    pub fn page_count(&self) -> u16 {
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
        self.document
            .pages()
            .get(index)
            .map_err(|_| PdfError::InvalidPageIndex(index))
    }

    /// Get the document's metadata
    pub fn metadata(&self) -> PdfMetadata {
        let meta = self.document.metadata();

        PdfMetadata {
            title: meta.get(PdfDocumentMetadataTagType::Title)
                .map(|v| v.value().to_string()),
            author: meta.get(PdfDocumentMetadataTagType::Author)
                .map(|v| v.value().to_string()),
            subject: meta.get(PdfDocumentMetadataTagType::Subject)
                .map(|v| v.value().to_string()),
            creator: meta.get(PdfDocumentMetadataTagType::Creator)
                .map(|v| v.value().to_string()),
            producer: meta.get(PdfDocumentMetadataTagType::Producer)
                .map(|v| v.value().to_string()),
        }
    }
}

/// PDF document metadata
#[derive(Debug, Clone, Default)]
pub struct PdfMetadata {
    pub title: Option<String>,
    pub author: Option<String>,
    pub subject: Option<String>,
    pub creator: Option<String>,
    pub producer: Option<String>,
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
}
