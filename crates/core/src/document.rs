//! Document state model and management
//!
//! Provides fast document loading with metadata-only initialization
//! and lazy loading of page content.

use crate::annotation::SerializableAnnotation;
use crate::measurement::{ScaleSystem, ScaleSystemId, SerializableMeasurement};
use crate::text_edit::PageTextEdits;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// Unique identifier for a document
pub type DocumentId = u64;

/// Page dimensions in points (1/72 inch)
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct PageDimensions {
    pub width: f32,
    pub height: f32,
}

/// Document metadata loaded during fast file open
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DocumentMetadata {
    /// Document title (from PDF metadata)
    pub title: Option<String>,

    /// Document author (from PDF metadata)
    pub author: Option<String>,

    /// Document subject (from PDF metadata)
    pub subject: Option<String>,

    /// Document creator (from PDF metadata)
    pub creator: Option<String>,

    /// Document producer (from PDF metadata)
    pub producer: Option<String>,

    /// Number of pages in the document
    pub page_count: u16,

    /// File path of the document
    pub file_path: PathBuf,

    /// File size in bytes
    pub file_size: u64,

    /// Cached page dimensions (page_index -> dimensions)
    /// This avoids needing to reopen the PDF just to get page sizes
    #[serde(default)]
    pub page_dimensions: std::collections::HashMap<u16, PageDimensions>,

    /// Scale systems for measurements (per-page)
    #[serde(default)]
    pub scale_systems: Vec<ScaleSystem>,

    /// Default scale system per page (page_index -> scale_system_id)
    #[serde(default)]
    pub default_scales: std::collections::HashMap<u16, ScaleSystemId>,

    /// Text edits for the document (per-page)
    #[serde(default)]
    pub text_edits: Vec<PageTextEdits>,

    /// Annotations for the document
    #[serde(default)]
    pub annotations: Vec<SerializableAnnotation>,

    /// Measurements for the document
    #[serde(default)]
    pub measurements: Vec<SerializableMeasurement>,
}

impl DocumentMetadata {
    /// Add a scale system to the document metadata
    pub fn add_scale_system(&mut self, scale: ScaleSystem) -> ScaleSystemId {
        let id = scale.id();
        let page_index = scale.page_index();
        self.scale_systems.push(scale);

        // Set as default for page if none exists
        self.default_scales.entry(page_index).or_insert(id);

        id
    }

    /// Get all scale systems for a specific page
    pub fn get_scales_for_page(&self, page_index: u16) -> Vec<&ScaleSystem> {
        self.scale_systems
            .iter()
            .filter(|s| s.page_index() == page_index)
            .collect()
    }

    /// Get a scale system by ID
    pub fn get_scale_by_id(&self, id: ScaleSystemId) -> Option<&ScaleSystem> {
        self.scale_systems.iter().find(|s| s.id() == id)
    }

    /// Get the default scale system for a page
    pub fn get_default_scale(&self, page_index: u16) -> Option<&ScaleSystem> {
        self.default_scales
            .get(&page_index)
            .and_then(|id| self.get_scale_by_id(*id))
    }

    /// Set the default scale system for a page
    pub fn set_default_scale(&mut self, page_index: u16, scale_id: ScaleSystemId) {
        if self.scale_systems.iter().any(|s| s.id() == scale_id) {
            self.default_scales.insert(page_index, scale_id);
        }
    }

    /// Remove a scale system by ID
    pub fn remove_scale_system(&mut self, id: ScaleSystemId) {
        self.scale_systems.retain(|s| s.id() != id);
        // Clear any default scale references
        self.default_scales.retain(|_, scale_id| *scale_id != id);
    }

    /// Get text edits for a specific page
    pub fn get_text_edits_for_page(&self, page_index: u16) -> Option<&PageTextEdits> {
        self.text_edits.iter().find(|e| e.page_index == page_index)
    }

    /// Get mutable text edits for a specific page
    pub fn get_text_edits_for_page_mut(&mut self, page_index: u16) -> Option<&mut PageTextEdits> {
        self.text_edits
            .iter_mut()
            .find(|e| e.page_index == page_index)
    }

    /// Set text edits for a specific page
    pub fn set_text_edits_for_page(&mut self, page_edits: PageTextEdits) {
        // Remove existing edits for this page
        self.text_edits
            .retain(|e| e.page_index != page_edits.page_index);

        // Only add if there are edits
        if !page_edits.edits.is_empty() {
            self.text_edits.push(page_edits);
        }
    }

    /// Clear all text edits for a specific page
    pub fn clear_text_edits_for_page(&mut self, page_index: u16) {
        self.text_edits.retain(|e| e.page_index != page_index);
    }

    /// Get total number of text edits across all pages
    pub fn total_text_edit_count(&self) -> usize {
        self.text_edits.iter().map(|p| p.edits.len()).sum()
    }

    /// Add an annotation to the document
    pub fn add_annotation(&mut self, annotation: SerializableAnnotation) {
        self.annotations.push(annotation);
    }

    /// Get all annotations for a specific page
    pub fn get_annotations_for_page(&self, page_index: u16) -> Vec<&SerializableAnnotation> {
        self.annotations
            .iter()
            .filter(|a| a.page_index == page_index)
            .collect()
    }

    /// Remove an annotation by ID
    pub fn remove_annotation(&mut self, id: crate::annotation::AnnotationId) -> bool {
        let initial_len = self.annotations.len();
        self.annotations.retain(|a| a.id != id);
        self.annotations.len() != initial_len
    }

    /// Get annotation by ID
    pub fn get_annotation(
        &self,
        id: crate::annotation::AnnotationId,
    ) -> Option<&SerializableAnnotation> {
        self.annotations.iter().find(|a| a.id == id)
    }

    /// Get total number of annotations across all pages
    pub fn total_annotation_count(&self) -> usize {
        self.annotations.len()
    }
}

impl Default for DocumentMetadata {
    fn default() -> Self {
        Self {
            title: None,
            author: None,
            subject: None,
            creator: None,
            producer: None,
            page_count: 0,
            file_path: PathBuf::new(),
            file_size: 0,
            page_dimensions: std::collections::HashMap::new(),
            scale_systems: Vec::new(),
            default_scales: std::collections::HashMap::new(),
            text_edits: Vec::new(),
            annotations: Vec::new(),
            measurements: Vec::new(),
        }
    }
}

/// Document loading state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentState {
    /// Document is being loaded (metadata only)
    Loading,

    /// Document metadata loaded, ready for rendering
    Ready,

    /// Document failed to load
    Error,

    /// Document is closed
    Closed,
}

/// Document handle with lazy loading support
///
/// Provides fast file open by loading only metadata initially.
/// Page content is rendered on-demand through the render pipeline.
pub struct Document {
    /// Unique document identifier
    id: DocumentId,

    /// Document metadata (loaded immediately)
    metadata: DocumentMetadata,

    /// Current document state
    state: Arc<Mutex<DocumentState>>,

    /// Current page index (zero-based)
    current_page: Arc<Mutex<u16>>,
}

impl Document {
    /// Create a new document handle with metadata
    ///
    /// This is called after fast metadata loading completes.
    pub fn new(id: DocumentId, metadata: DocumentMetadata) -> Self {
        Self {
            id,
            metadata,
            state: Arc::new(Mutex::new(DocumentState::Ready)),
            current_page: Arc::new(Mutex::new(0)),
        }
    }

    /// Get the document ID
    pub fn id(&self) -> DocumentId {
        self.id
    }

    /// Get the document metadata
    pub fn metadata(&self) -> &DocumentMetadata {
        &self.metadata
    }

    /// Get the document state
    pub fn state(&self) -> DocumentState {
        *self.state.lock().unwrap()
    }

    /// Set the document state
    pub fn set_state(&self, state: DocumentState) {
        *self.state.lock().unwrap() = state;
    }

    /// Get the current page index (zero-based)
    pub fn current_page(&self) -> u16 {
        *self.current_page.lock().unwrap()
    }

    /// Set the current page index (zero-based)
    ///
    /// Returns true if the page index was changed, false if out of bounds.
    pub fn set_current_page(&self, page_index: u16) -> bool {
        if page_index >= self.metadata.page_count {
            return false;
        }
        *self.current_page.lock().unwrap() = page_index;
        true
    }

    /// Navigate to the next page
    ///
    /// Returns true if navigation succeeded, false if already on last page.
    pub fn next_page(&self) -> bool {
        let mut current = self.current_page.lock().unwrap();
        if *current + 1 < self.metadata.page_count {
            *current += 1;
            true
        } else {
            false
        }
    }

    /// Navigate to the previous page
    ///
    /// Returns true if navigation succeeded, false if already on first page.
    pub fn prev_page(&self) -> bool {
        let mut current = self.current_page.lock().unwrap();
        if *current > 0 {
            *current -= 1;
            true
        } else {
            false
        }
    }

    /// Get the page count
    pub fn page_count(&self) -> u16 {
        self.metadata.page_count
    }

    /// Check if this is the first page
    pub fn is_first_page(&self) -> bool {
        self.current_page() == 0
    }

    /// Check if this is the last page
    pub fn is_last_page(&self) -> bool {
        self.current_page() + 1 >= self.metadata.page_count
    }

    /// Get mutable access to metadata (for scale management)
    pub fn metadata_mut(&mut self) -> &mut DocumentMetadata {
        &mut self.metadata
    }
}

/// Errors that can occur during document management
#[derive(Debug)]
pub enum DocumentError {
    /// Document not found
    NotFound(DocumentId),

    /// Document already exists
    AlreadyExists(DocumentId),

    /// Failed to load document
    LoadError(String),

    /// Invalid page index
    InvalidPageIndex { page: u16, max: u16 },

    /// Document is closed
    Closed(DocumentId),
}

impl std::fmt::Display for DocumentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DocumentError::NotFound(id) => write!(f, "Document not found: {}", id),
            DocumentError::AlreadyExists(id) => write!(f, "Document already exists: {}", id),
            DocumentError::LoadError(msg) => write!(f, "Failed to load document: {}", msg),
            DocumentError::InvalidPageIndex { page, max } => {
                write!(f, "Invalid page index {} (max: {})", page, max)
            }
            DocumentError::Closed(id) => write!(f, "Document is closed: {}", id),
        }
    }
}

impl std::error::Error for DocumentError {}

/// Result type for document operations
pub type DocumentResult<T> = Result<T, DocumentError>;

/// Document manager for handling multiple open documents
///
/// Provides fast file opening with metadata-only loading.
/// Actual page rendering is deferred to the render pipeline.
pub struct DocumentManager {
    /// Map of document ID to document handle
    documents: Arc<Mutex<std::collections::HashMap<DocumentId, Document>>>,

    /// Counter for generating unique document IDs
    next_id: Arc<Mutex<DocumentId>>,

    /// Currently active document ID
    active_document: Arc<Mutex<Option<DocumentId>>>,
}

impl DocumentManager {
    /// Create a new document manager
    pub fn new() -> Self {
        Self {
            documents: Arc::new(Mutex::new(std::collections::HashMap::new())),
            next_id: Arc::new(Mutex::new(1)),
            active_document: Arc::new(Mutex::new(None)),
        }
    }

    /// Register a new document with the manager
    ///
    /// This is called after fast metadata loading completes.
    /// Returns the document ID.
    pub fn register_document(&self, metadata: DocumentMetadata) -> DocumentId {
        let mut next_id = self.next_id.lock().unwrap();
        let id = *next_id;
        *next_id += 1;
        drop(next_id);

        let document = Document::new(id, metadata);
        self.documents.lock().unwrap().insert(id, document);

        // Set as active if this is the first document
        let mut active = self.active_document.lock().unwrap();
        if active.is_none() {
            *active = Some(id);
        }

        id
    }

    /// Get a document by ID
    pub fn get_document(&self, id: DocumentId) -> DocumentResult<Document> {
        self.documents
            .lock()
            .unwrap()
            .get(&id)
            .cloned()
            .ok_or(DocumentError::NotFound(id))
    }

    /// Get the currently active document
    pub fn active_document(&self) -> Option<Document> {
        let active_id = *self.active_document.lock().unwrap();
        active_id.and_then(|id| self.documents.lock().unwrap().get(&id).cloned())
    }

    /// Set the active document
    pub fn set_active_document(&self, id: DocumentId) -> DocumentResult<()> {
        if !self.documents.lock().unwrap().contains_key(&id) {
            return Err(DocumentError::NotFound(id));
        }
        *self.active_document.lock().unwrap() = Some(id);
        Ok(())
    }

    /// Close a document
    pub fn close_document(&self, id: DocumentId) -> DocumentResult<()> {
        let mut documents = self.documents.lock().unwrap();
        let document = documents.get(&id).ok_or(DocumentError::NotFound(id))?;

        document.set_state(DocumentState::Closed);
        documents.remove(&id);

        // Clear active document if this was active
        let mut active = self.active_document.lock().unwrap();
        if *active == Some(id) {
            *active = None;
        }

        Ok(())
    }

    /// Get all open document IDs
    pub fn open_documents(&self) -> Vec<DocumentId> {
        self.documents.lock().unwrap().keys().copied().collect()
    }

    /// Get the number of open documents
    pub fn document_count(&self) -> usize {
        self.documents.lock().unwrap().len()
    }

    /// Check if a document is open
    pub fn is_open(&self, id: DocumentId) -> bool {
        self.documents.lock().unwrap().contains_key(&id)
    }
}

impl Default for DocumentManager {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for Document {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            metadata: self.metadata.clone(),
            state: Arc::clone(&self.state),
            current_page: Arc::clone(&self.current_page),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_metadata() -> DocumentMetadata {
        DocumentMetadata {
            title: Some("Test Document".to_string()),
            author: Some("Test Author".to_string()),
            subject: Some("Test Subject".to_string()),
            creator: Some("Test Creator".to_string()),
            producer: Some("Test Producer".to_string()),
            page_count: 10,
            file_path: PathBuf::from("/test/document.pdf"),
            file_size: 1024,
            page_dimensions: std::collections::HashMap::new(),
            scale_systems: Vec::new(),
            default_scales: std::collections::HashMap::new(),
            text_edits: Vec::new(),
            annotations: Vec::new(),
            measurements: Vec::new(),
        }
    }

    #[test]
    fn test_document_metadata_default() {
        let metadata = DocumentMetadata::default();
        assert_eq!(metadata.page_count, 0);
        assert_eq!(metadata.file_size, 0);
        assert!(metadata.title.is_none());
    }

    #[test]
    fn test_document_creation() {
        let metadata = test_metadata();
        let doc = Document::new(1, metadata);

        assert_eq!(doc.id(), 1);
        assert_eq!(doc.state(), DocumentState::Ready);
        assert_eq!(doc.current_page(), 0);
        assert_eq!(doc.page_count(), 10);
    }

    #[test]
    fn test_document_state_management() {
        let metadata = test_metadata();
        let doc = Document::new(1, metadata);

        assert_eq!(doc.state(), DocumentState::Ready);

        doc.set_state(DocumentState::Loading);
        assert_eq!(doc.state(), DocumentState::Loading);

        doc.set_state(DocumentState::Error);
        assert_eq!(doc.state(), DocumentState::Error);
    }

    #[test]
    fn test_document_page_navigation() {
        let metadata = test_metadata();
        let doc = Document::new(1, metadata);

        // Initial state
        assert_eq!(doc.current_page(), 0);
        assert!(doc.is_first_page());
        assert!(!doc.is_last_page());

        // Navigate forward
        assert!(doc.next_page());
        assert_eq!(doc.current_page(), 1);
        assert!(!doc.is_first_page());

        // Navigate backward
        assert!(doc.prev_page());
        assert_eq!(doc.current_page(), 0);
        assert!(doc.is_first_page());

        // Can't go before first page
        assert!(!doc.prev_page());
        assert_eq!(doc.current_page(), 0);

        // Jump to last page
        assert!(doc.set_current_page(9));
        assert_eq!(doc.current_page(), 9);
        assert!(doc.is_last_page());

        // Can't go past last page
        assert!(!doc.next_page());
        assert_eq!(doc.current_page(), 9);
    }

    #[test]
    fn test_document_set_current_page() {
        let metadata = test_metadata();
        let doc = Document::new(1, metadata);

        // Valid page index
        assert!(doc.set_current_page(5));
        assert_eq!(doc.current_page(), 5);

        // Invalid page index (out of bounds)
        assert!(!doc.set_current_page(10));
        assert_eq!(doc.current_page(), 5); // Should remain unchanged
    }

    #[test]
    fn test_document_manager_creation() {
        let manager = DocumentManager::new();
        assert_eq!(manager.document_count(), 0);
        assert!(manager.active_document().is_none());
    }

    #[test]
    fn test_document_manager_register() {
        let manager = DocumentManager::new();
        let metadata = test_metadata();

        let id1 = manager.register_document(metadata.clone());
        assert_eq!(id1, 1);
        assert_eq!(manager.document_count(), 1);

        let id2 = manager.register_document(metadata);
        assert_eq!(id2, 2);
        assert_eq!(manager.document_count(), 2);
    }

    #[test]
    fn test_document_manager_get_document() {
        let manager = DocumentManager::new();
        let metadata = test_metadata();
        let id = manager.register_document(metadata);

        let doc = manager.get_document(id).unwrap();
        assert_eq!(doc.id(), id);
        assert_eq!(doc.page_count(), 10);
    }

    #[test]
    fn test_document_manager_get_document_not_found() {
        let manager = DocumentManager::new();
        let result = manager.get_document(999);
        assert!(result.is_err());
    }

    #[test]
    fn test_document_manager_active_document() {
        let manager = DocumentManager::new();

        // No active document initially
        assert!(manager.active_document().is_none());

        // First registered document becomes active
        let metadata = test_metadata();
        let id1 = manager.register_document(metadata.clone());
        assert_eq!(manager.active_document().unwrap().id(), id1);

        // Register another document
        let id2 = manager.register_document(metadata);
        assert_eq!(manager.active_document().unwrap().id(), id1); // Still first

        // Change active document
        manager.set_active_document(id2).unwrap();
        assert_eq!(manager.active_document().unwrap().id(), id2);
    }

    #[test]
    fn test_document_manager_close_document() {
        let manager = DocumentManager::new();
        let metadata = test_metadata();
        let id = manager.register_document(metadata);

        assert_eq!(manager.document_count(), 1);
        assert!(manager.is_open(id));

        manager.close_document(id).unwrap();
        assert_eq!(manager.document_count(), 0);
        assert!(!manager.is_open(id));
        assert!(manager.active_document().is_none());
    }

    #[test]
    fn test_document_manager_open_documents() {
        let manager = DocumentManager::new();
        let metadata = test_metadata();

        let id1 = manager.register_document(metadata.clone());
        let id2 = manager.register_document(metadata.clone());
        let id3 = manager.register_document(metadata);

        let open_docs = manager.open_documents();
        assert_eq!(open_docs.len(), 3);
        assert!(open_docs.contains(&id1));
        assert!(open_docs.contains(&id2));
        assert!(open_docs.contains(&id3));
    }

    #[test]
    fn test_document_clone() {
        let metadata = test_metadata();
        let doc = Document::new(1, metadata);

        doc.set_current_page(5);

        let cloned = doc.clone();
        assert_eq!(cloned.id(), doc.id());
        assert_eq!(cloned.current_page(), doc.current_page());

        // Verify shared state
        cloned.set_current_page(7);
        assert_eq!(doc.current_page(), 7); // Original should also see change
    }

    #[test]
    fn test_document_error_display() {
        let err1 = DocumentError::NotFound(123);
        assert_eq!(err1.to_string(), "Document not found: 123");

        let err2 = DocumentError::InvalidPageIndex { page: 5, max: 3 };
        assert_eq!(err2.to_string(), "Invalid page index 5 (max: 3)");
    }

    #[test]
    fn test_document_metadata_add_scale_system() {
        use crate::measurement::ScaleSystem;

        let mut metadata = test_metadata();
        let scale = ScaleSystem::manual(0, 72.0, "inches");
        let scale_id = scale.id();

        let returned_id = metadata.add_scale_system(scale);
        assert_eq!(returned_id, scale_id);
        assert_eq!(metadata.scale_systems.len(), 1);
        assert_eq!(metadata.get_default_scale(0).unwrap().id(), scale_id);
    }

    #[test]
    fn test_document_metadata_get_scales_for_page() {
        use crate::measurement::ScaleSystem;

        let mut metadata = test_metadata();
        let scale1 = ScaleSystem::manual(0, 72.0, "inches");
        let scale2 = ScaleSystem::manual(0, 36.0, "inches");
        let scale3 = ScaleSystem::manual(1, 100.0, "cm");

        metadata.add_scale_system(scale1);
        metadata.add_scale_system(scale2);
        metadata.add_scale_system(scale3);

        let page0_scales = metadata.get_scales_for_page(0);
        assert_eq!(page0_scales.len(), 2);

        let page1_scales = metadata.get_scales_for_page(1);
        assert_eq!(page1_scales.len(), 1);
    }

    #[test]
    fn test_document_metadata_set_default_scale() {
        use crate::measurement::ScaleSystem;

        let mut metadata = test_metadata();
        let scale1 = ScaleSystem::manual(0, 72.0, "inches");
        let scale2 = ScaleSystem::manual(0, 36.0, "inches");

        let id1 = metadata.add_scale_system(scale1);
        let id2 = metadata.add_scale_system(scale2);

        // First scale should be default
        assert_eq!(metadata.get_default_scale(0).unwrap().id(), id1);

        // Change default
        metadata.set_default_scale(0, id2);
        assert_eq!(metadata.get_default_scale(0).unwrap().id(), id2);
    }

    #[test]
    fn test_document_metadata_remove_scale_system() {
        use crate::measurement::ScaleSystem;

        let mut metadata = test_metadata();
        let scale = ScaleSystem::manual(0, 72.0, "inches");
        let scale_id = metadata.add_scale_system(scale);

        assert_eq!(metadata.scale_systems.len(), 1);
        assert!(metadata.get_default_scale(0).is_some());

        metadata.remove_scale_system(scale_id);

        assert_eq!(metadata.scale_systems.len(), 0);
        assert!(metadata.get_default_scale(0).is_none());
    }

    #[test]
    fn test_document_metadata_serialization() {
        use crate::measurement::ScaleSystem;

        let mut metadata = test_metadata();
        let scale = ScaleSystem::manual(0, 72.0, "inches");
        metadata.add_scale_system(scale);

        // Serialize
        let json = serde_json::to_string(&metadata).unwrap();

        // Deserialize
        let deserialized: DocumentMetadata = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.title, metadata.title);
        assert_eq!(deserialized.page_count, metadata.page_count);
        assert_eq!(deserialized.scale_systems.len(), 1);
        assert_eq!(deserialized.scale_systems[0].unit(), "inches");
        assert_eq!(deserialized.scale_systems[0].ratio(), 72.0);
    }
}
