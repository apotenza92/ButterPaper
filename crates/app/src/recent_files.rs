//! Recent Files Management
//!
//! This module tracks recently opened PDF files and persists them to disk.
//! The list is used to populate the "Open Recent" submenu in the File menu.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

/// Maximum number of recent files to track
const MAX_RECENT_FILES: usize = 10;

/// Global storage for recent files (thread-safe singleton)
static RECENT_FILES: std::sync::OnceLock<Arc<RwLock<RecentFiles>>> = std::sync::OnceLock::new();

/// Get or initialize the global recent files manager
pub fn get_recent_files() -> Arc<RwLock<RecentFiles>> {
    RECENT_FILES
        .get_or_init(|| {
            let mut recent = RecentFiles::new();
            if let Err(e) = recent.load() {
                eprintln!("Warning: Could not load recent files: {}", e);
            }
            Arc::new(RwLock::new(recent))
        })
        .clone()
}

/// Manages a list of recently opened files
#[derive(Debug, Clone)]
pub struct RecentFiles {
    /// List of recent file paths (most recent first)
    files: Vec<PathBuf>,
    /// Path to the persistence file
    storage_path: PathBuf,
}

impl RecentFiles {
    /// Creates a new RecentFiles manager
    pub fn new() -> Self {
        Self {
            files: Vec::new(),
            storage_path: Self::default_storage_path(),
        }
    }

    /// Creates a RecentFiles manager with a custom storage path (for testing)
    #[cfg(test)]
    pub fn with_storage_path<P: AsRef<Path>>(path: P) -> Self {
        Self {
            files: Vec::new(),
            storage_path: path.as_ref().to_path_buf(),
        }
    }

    /// Returns the default storage path for recent files
    ///
    /// - macOS: ~/Library/Application Support/pdf-editor/recent_files.json
    /// - Linux: ~/.local/share/pdf-editor/recent_files.json
    /// - Windows: %APPDATA%\pdf-editor\recent_files.json
    fn default_storage_path() -> PathBuf {
        if let Some(data_dir) = dirs::data_dir() {
            data_dir.join("pdf-editor").join("recent_files.json")
        } else {
            // Fallback to current directory
            PathBuf::from("recent_files.json")
        }
    }

    /// Adds a file to the recent files list
    ///
    /// If the file already exists in the list, it is moved to the front.
    /// The list is capped at MAX_RECENT_FILES entries.
    pub fn add<P: AsRef<Path>>(&mut self, path: P) {
        let path = path.as_ref().to_path_buf();

        // Remove if already present (to move to front)
        self.files.retain(|p| p != &path);

        // Add to front
        self.files.insert(0, path);

        // Cap at max entries
        self.files.truncate(MAX_RECENT_FILES);
    }

    /// Returns the list of recent files (most recent first)
    pub fn files(&self) -> &[PathBuf] {
        &self.files
    }

    /// Clears all recent files
    pub fn clear(&mut self) {
        self.files.clear();
    }

    /// Loads recent files from disk
    pub fn load(&mut self) -> Result<(), RecentFilesError> {
        if !self.storage_path.exists() {
            return Ok(());
        }

        let contents = fs::read_to_string(&self.storage_path)
            .map_err(RecentFilesError::IoError)?;

        self.files = Self::parse_json(&contents)?;

        // Filter out files that no longer exist
        self.files.retain(|p| p.exists());

        Ok(())
    }

    /// Saves recent files to disk
    pub fn save(&self) -> Result<(), RecentFilesError> {
        // Ensure parent directory exists
        if let Some(parent) = self.storage_path.parent() {
            fs::create_dir_all(parent).map_err(RecentFilesError::IoError)?;
        }

        let json = self.to_json();
        fs::write(&self.storage_path, json).map_err(RecentFilesError::IoError)
    }

    /// Parses JSON array of file paths
    fn parse_json(json: &str) -> Result<Vec<PathBuf>, RecentFilesError> {
        // Simple JSON array parser (no external dependencies)
        let json = json.trim();
        if !json.starts_with('[') || !json.ends_with(']') {
            return Err(RecentFilesError::ParseError("Invalid JSON array".to_string()));
        }

        let inner = &json[1..json.len() - 1];
        if inner.trim().is_empty() {
            return Ok(Vec::new());
        }

        let mut files = Vec::new();
        let mut current = String::new();
        let mut in_string = false;
        let mut escape_next = false;

        for c in inner.chars() {
            if escape_next {
                current.push(c);
                escape_next = false;
                continue;
            }

            match c {
                '\\' if in_string => escape_next = true,
                '"' => {
                    if in_string {
                        files.push(PathBuf::from(&current));
                        current.clear();
                    }
                    in_string = !in_string;
                }
                ',' if !in_string => {
                    // Skip comma separators
                }
                _ if in_string => current.push(c),
                _ => {
                    // Skip whitespace outside strings
                }
            }
        }

        Ok(files)
    }

    /// Converts recent files to JSON array
    fn to_json(&self) -> String {
        let paths: Vec<String> = self
            .files
            .iter()
            .map(|p| {
                // Escape backslashes and quotes in paths
                let s = p.display().to_string();
                let escaped = s.replace('\\', "\\\\").replace('"', "\\\"");
                format!("\"{}\"", escaped)
            })
            .collect();

        format!("[\n  {}\n]", paths.join(",\n  "))
    }
}

impl Default for RecentFiles {
    fn default() -> Self {
        Self::new()
    }
}

/// Errors that can occur during recent files operations
#[derive(Debug)]
pub enum RecentFilesError {
    /// I/O error reading or writing files
    IoError(io::Error),
    /// Parse error reading JSON
    ParseError(String),
}

impl std::fmt::Display for RecentFilesError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RecentFilesError::IoError(e) => write!(f, "I/O error: {}", e),
            RecentFilesError::ParseError(msg) => write!(f, "Parse error: {}", msg),
        }
    }
}

impl std::error::Error for RecentFilesError {}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_add_file() {
        let mut recent = RecentFiles::new();
        recent.add("/path/to/file1.pdf");
        recent.add("/path/to/file2.pdf");

        assert_eq!(recent.files().len(), 2);
        assert_eq!(recent.files()[0], PathBuf::from("/path/to/file2.pdf"));
        assert_eq!(recent.files()[1], PathBuf::from("/path/to/file1.pdf"));
    }

    #[test]
    fn test_add_duplicate_moves_to_front() {
        let mut recent = RecentFiles::new();
        recent.add("/path/to/file1.pdf");
        recent.add("/path/to/file2.pdf");
        recent.add("/path/to/file1.pdf"); // Re-add file1

        assert_eq!(recent.files().len(), 2);
        assert_eq!(recent.files()[0], PathBuf::from("/path/to/file1.pdf"));
        assert_eq!(recent.files()[1], PathBuf::from("/path/to/file2.pdf"));
    }

    #[test]
    fn test_max_files_limit() {
        let mut recent = RecentFiles::new();

        for i in 0..15 {
            recent.add(format!("/path/to/file{}.pdf", i));
        }

        assert_eq!(recent.files().len(), MAX_RECENT_FILES);
        // Most recent should be last added
        assert_eq!(recent.files()[0], PathBuf::from("/path/to/file14.pdf"));
    }

    #[test]
    fn test_clear() {
        let mut recent = RecentFiles::new();
        recent.add("/path/to/file1.pdf");
        recent.add("/path/to/file2.pdf");

        recent.clear();

        assert!(recent.files().is_empty());
    }

    #[test]
    fn test_json_roundtrip() {
        let mut recent = RecentFiles::new();
        recent.add("/path/to/file1.pdf");
        recent.add("/path/with spaces/file2.pdf");
        recent.add("/path/with\"quotes\"/file3.pdf");

        let json = recent.to_json();
        let parsed = RecentFiles::parse_json(&json).unwrap();

        assert_eq!(recent.files().len(), parsed.len());
        for (a, b) in recent.files().iter().zip(parsed.iter()) {
            assert_eq!(a, b);
        }
    }

    #[test]
    fn test_parse_empty_json() {
        let files = RecentFiles::parse_json("[]").unwrap();
        assert!(files.is_empty());

        let files = RecentFiles::parse_json("[  ]").unwrap();
        assert!(files.is_empty());
    }

    #[test]
    fn test_parse_invalid_json() {
        let result = RecentFiles::parse_json("not json");
        assert!(result.is_err());
    }

    #[test]
    fn test_save_and_load() {
        let temp_dir = TempDir::new().unwrap();
        let storage_path = temp_dir.path().join("recent_files.json");

        // Save
        let mut recent = RecentFiles::with_storage_path(&storage_path);
        recent.add(temp_dir.path().join("existing.pdf"));
        recent.save().unwrap();

        // Create the "existing" file so it passes the exists() check
        fs::write(temp_dir.path().join("existing.pdf"), b"fake pdf").unwrap();

        // Load
        let mut loaded = RecentFiles::with_storage_path(&storage_path);
        loaded.load().unwrap();

        assert_eq!(loaded.files().len(), 1);
    }

    #[test]
    fn test_load_filters_nonexistent_files() {
        let temp_dir = TempDir::new().unwrap();
        let storage_path = temp_dir.path().join("recent_files.json");

        // Write JSON with a file that doesn't exist
        let json = r#"["/nonexistent/file.pdf"]"#;
        fs::write(&storage_path, json).unwrap();

        let mut recent = RecentFiles::with_storage_path(&storage_path);
        recent.load().unwrap();

        // Should be empty since the file doesn't exist
        assert!(recent.files().is_empty());
    }

    #[test]
    fn test_load_nonexistent_storage_file() {
        let temp_dir = TempDir::new().unwrap();
        let storage_path = temp_dir.path().join("nonexistent.json");

        let mut recent = RecentFiles::with_storage_path(&storage_path);
        let result = recent.load();

        assert!(result.is_ok());
        assert!(recent.files().is_empty());
    }

    #[test]
    fn test_default_storage_path() {
        let path = RecentFiles::default_storage_path();
        // Should contain "pdf-editor" in the path
        assert!(path.to_string_lossy().contains("pdf-editor"));
    }
}
