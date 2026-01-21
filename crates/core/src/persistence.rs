//! Document persistence for metadata and annotations
//!
//! Provides utilities to save and load document metadata (including scale systems)
//! as JSON sidecar files alongside the PDF.

use crate::document::DocumentMetadata;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Error types for persistence operations
#[derive(Debug)]
pub enum PersistenceError {
    /// IO error during file operations
    IoError(io::Error),
    /// Serialization error
    SerializationError(String),
    /// Deserialization error
    DeserializationError(String),
}

impl std::fmt::Display for PersistenceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PersistenceError::IoError(e) => write!(f, "IO error: {}", e),
            PersistenceError::SerializationError(e) => write!(f, "Serialization error: {}", e),
            PersistenceError::DeserializationError(e) => write!(f, "Deserialization error: {}", e),
        }
    }
}

impl std::error::Error for PersistenceError {}

impl From<io::Error> for PersistenceError {
    fn from(err: io::Error) -> Self {
        PersistenceError::IoError(err)
    }
}

/// Result type for persistence operations
pub type PersistenceResult<T> = Result<T, PersistenceError>;

/// Get the metadata file path for a given PDF path
///
/// The metadata is stored as a JSON sidecar file with the same name
/// but with a `.pdf-editor-metadata.json` extension.
///
/// # Example
/// ```
/// use std::path::Path;
/// use pdf_editor_core::persistence::metadata_path;
///
/// let pdf_path = Path::new("/path/to/document.pdf");
/// let meta_path = metadata_path(pdf_path);
/// assert_eq!(meta_path, Path::new("/path/to/document.pdf.pdf-editor-metadata.json"));
/// ```
pub fn metadata_path(pdf_path: &Path) -> PathBuf {
    // Append the metadata extension to the full PDF filename
    let mut path_str = pdf_path.to_string_lossy().to_string();
    path_str.push_str(".pdf-editor-metadata.json");
    PathBuf::from(path_str)
}

/// Save document metadata to a JSON sidecar file
///
/// # Arguments
/// * `metadata` - The document metadata to save
///
/// # Returns
/// The path to the saved metadata file
///
/// # Errors
/// Returns `PersistenceError` if serialization or file write fails
pub fn save_metadata(metadata: &DocumentMetadata) -> PersistenceResult<PathBuf> {
    let meta_path = metadata_path(&metadata.file_path);

    // Serialize to JSON with pretty printing
    let json = serde_json::to_string_pretty(metadata)
        .map_err(|e| PersistenceError::SerializationError(e.to_string()))?;

    // Write to file atomically using a temporary file
    let temp_path = meta_path.with_extension("tmp");
    fs::write(&temp_path, json)?;
    fs::rename(&temp_path, &meta_path)?;

    Ok(meta_path)
}

/// Load document metadata from a JSON sidecar file
///
/// # Arguments
/// * `pdf_path` - Path to the PDF file
///
/// # Returns
/// The loaded document metadata, or None if no metadata file exists
///
/// # Errors
/// Returns `PersistenceError` if deserialization fails
pub fn load_metadata(pdf_path: &Path) -> PersistenceResult<Option<DocumentMetadata>> {
    let meta_path = metadata_path(pdf_path);

    // Check if metadata file exists
    if !meta_path.exists() {
        return Ok(None);
    }

    // Read and deserialize
    let json = fs::read_to_string(&meta_path)?;
    let metadata: DocumentMetadata = serde_json::from_str(&json)
        .map_err(|e| PersistenceError::DeserializationError(e.to_string()))?;

    Ok(Some(metadata))
}

/// Check if metadata exists for a PDF file
pub fn metadata_exists(pdf_path: &Path) -> bool {
    metadata_path(pdf_path).exists()
}

/// Delete metadata file for a PDF
pub fn delete_metadata(pdf_path: &Path) -> PersistenceResult<()> {
    let meta_path = metadata_path(pdf_path);
    if meta_path.exists() {
        fs::remove_file(meta_path)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::measurement::ScaleSystem;
    use std::path::PathBuf;

    fn test_metadata() -> DocumentMetadata {
        // Use temp directory
        let temp_dir = std::env::temp_dir();
        let pdf_path = temp_dir.join("test_persistence.pdf");

        let mut metadata = DocumentMetadata {
            title: Some("Test Document".to_string()),
            author: Some("Test Author".to_string()),
            subject: None,
            creator: None,
            producer: None,
            page_count: 5,
            file_path: pdf_path,
            file_size: 1024,
            scale_systems: Vec::new(),
            default_scales: std::collections::HashMap::new(),
            text_edits: Vec::new(),
        };

        // Add a scale system
        let scale = ScaleSystem::manual(0, 72.0, "inches");
        metadata.add_scale_system(scale);

        metadata
    }

    #[test]
    fn test_metadata_path() {
        let pdf_path = Path::new("/path/to/document.pdf");
        let meta_path = metadata_path(pdf_path);
        assert_eq!(
            meta_path,
            PathBuf::from("/path/to/document.pdf.pdf-editor-metadata.json")
        );
    }

    #[test]
    fn test_save_and_load_metadata() {
        let temp_dir = std::env::temp_dir();
        let pdf_path = temp_dir.join("test_save_and_load.pdf");

        let mut metadata = test_metadata();
        metadata.file_path = pdf_path.clone();

        // Save
        let saved_path = save_metadata(&metadata).unwrap();
        assert!(saved_path.exists());

        // Load
        let loaded = load_metadata(&metadata.file_path).unwrap();
        assert!(loaded.is_some());

        let loaded_metadata = loaded.unwrap();
        assert_eq!(loaded_metadata.title, metadata.title);
        assert_eq!(loaded_metadata.author, metadata.author);
        assert_eq!(loaded_metadata.page_count, metadata.page_count);
        assert_eq!(loaded_metadata.scale_systems.len(), 1);
        assert_eq!(loaded_metadata.scale_systems[0].unit(), "inches");

        // Cleanup
        delete_metadata(&metadata.file_path).unwrap();
    }

    #[test]
    fn test_metadata_exists() {
        let temp_dir = std::env::temp_dir();
        let pdf_path = temp_dir.join("test_exists.pdf");

        let mut metadata = test_metadata();
        metadata.file_path = pdf_path.clone();

        assert!(!metadata_exists(&metadata.file_path));

        save_metadata(&metadata).unwrap();
        assert!(metadata_exists(&metadata.file_path));

        delete_metadata(&metadata.file_path).unwrap();
        assert!(!metadata_exists(&metadata.file_path));
    }

    #[test]
    fn test_load_nonexistent_metadata() {
        let pdf_path = Path::new("/tmp/nonexistent.pdf");
        let result = load_metadata(pdf_path).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_delete_nonexistent_metadata() {
        let pdf_path = Path::new("/tmp/nonexistent.pdf");
        let result = delete_metadata(pdf_path);
        assert!(result.is_ok());
    }

    #[test]
    fn test_roundtrip_scale_systems() {
        let temp_dir = std::env::temp_dir();
        let pdf_path = temp_dir.join("test_roundtrip.pdf");

        let mut metadata = test_metadata();
        metadata.file_path = pdf_path.clone();

        // Add multiple scales
        let scale2 = ScaleSystem::manual(1, 100.0, "cm");
        metadata.add_scale_system(scale2);

        // Save and load
        save_metadata(&metadata).unwrap();
        let loaded = load_metadata(&metadata.file_path).unwrap().unwrap();

        assert_eq!(loaded.scale_systems.len(), 2);
        assert_eq!(loaded.scale_systems[0].page_index(), 0);
        assert_eq!(loaded.scale_systems[1].page_index(), 1);

        // Cleanup
        delete_metadata(&metadata.file_path).unwrap();
    }
}
