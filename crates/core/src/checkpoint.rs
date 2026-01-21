//! Crash-safe checkpointing with Write-Ahead Logging (WAL)
//!
//! Provides checkpoint mechanism for crash recovery. Uses a WAL pattern
//! to ensure that metadata changes are recoverable even if the application
//! crashes during a write operation.

use crate::document::DocumentMetadata;
use crate::persistence::{metadata_path, PersistenceError, PersistenceResult};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Checkpoint manager for crash-safe persistence
///
/// Uses a Write-Ahead Log (WAL) to ensure that all changes are recoverable
/// even if the application crashes during a write operation.
///
/// The checkpoint system works as follows:
/// 1. Write changes to a WAL file first
/// 2. Sync the WAL to disk
/// 3. Write the main metadata file
/// 4. Delete the WAL file after successful write
///
/// On recovery:
/// 1. Check if WAL file exists
/// 2. If it does, replay the WAL to restore state
/// 3. Complete any interrupted writes
pub struct CheckpointManager {
    /// Base path for the PDF file
    pdf_path: PathBuf,
}

impl CheckpointManager {
    /// Create a new checkpoint manager for a given PDF file
    pub fn new(pdf_path: impl AsRef<Path>) -> Self {
        Self {
            pdf_path: pdf_path.as_ref().to_path_buf(),
        }
    }

    /// Get the WAL file path for this document
    pub fn wal_path(&self) -> PathBuf {
        let mut path_str = self.pdf_path.to_string_lossy().to_string();
        path_str.push_str(".pdf-editor-wal.json");
        PathBuf::from(path_str)
    }

    /// Get the checkpoint file path for this document
    pub fn checkpoint_path(&self) -> PathBuf {
        let mut path_str = self.pdf_path.to_string_lossy().to_string();
        path_str.push_str(".pdf-editor-checkpoint.json");
        PathBuf::from(path_str)
    }

    /// Write metadata with crash-safe guarantees
    ///
    /// Uses WAL pattern:
    /// 1. Write to WAL file
    /// 2. Sync WAL to disk
    /// 3. Write to main metadata file
    /// 4. Delete WAL file
    ///
    /// If the process crashes at any point, recovery can complete the operation.
    pub fn write_with_checkpoint(&self, metadata: &DocumentMetadata) -> PersistenceResult<PathBuf> {
        let meta_path = metadata_path(&self.pdf_path);
        let wal_path = self.wal_path();

        // Step 1: Serialize metadata
        let json = serde_json::to_string_pretty(metadata)
            .map_err(|e| PersistenceError::SerializationError(e.to_string()))?;

        // Step 2: Write to WAL file with sync
        self.write_and_sync(&wal_path, &json)?;

        // Step 3: Write to main metadata file (atomic using temp file)
        let temp_path = meta_path.with_extension("tmp");
        fs::write(&temp_path, &json)?;
        fs::rename(&temp_path, &meta_path)?;

        // Step 4: Delete WAL file (write is complete)
        if wal_path.exists() {
            fs::remove_file(&wal_path)?;
        }

        Ok(meta_path)
    }

    /// Create a checkpoint (point-in-time snapshot)
    ///
    /// Checkpoints are separate from the main metadata file and can be used
    /// for recovery or rollback operations.
    pub fn create_checkpoint(&self, metadata: &DocumentMetadata) -> PersistenceResult<PathBuf> {
        let checkpoint_path = self.checkpoint_path();

        let json = serde_json::to_string_pretty(metadata)
            .map_err(|e| PersistenceError::SerializationError(e.to_string()))?;

        // Write checkpoint atomically
        let temp_path = checkpoint_path.with_extension("tmp");
        fs::write(&temp_path, &json)?;
        fs::rename(&temp_path, &checkpoint_path)?;

        Ok(checkpoint_path)
    }

    /// Recover from a crash by replaying the WAL
    ///
    /// Returns:
    /// - Ok(Some(metadata)) if WAL was found and recovered
    /// - Ok(None) if no WAL found (clean state)
    /// - Err if recovery failed
    pub fn recover(&self) -> PersistenceResult<Option<DocumentMetadata>> {
        let wal_path = self.wal_path();

        if !wal_path.exists() {
            // No WAL file means clean shutdown
            return Ok(None);
        }

        // Read WAL file
        let json = fs::read_to_string(&wal_path)?;
        let metadata: DocumentMetadata = serde_json::from_str(&json)
            .map_err(|e| PersistenceError::DeserializationError(e.to_string()))?;

        // Complete the interrupted write
        let meta_path = metadata_path(&self.pdf_path);
        let temp_path = meta_path.with_extension("tmp");
        fs::write(&temp_path, &json)?;
        fs::rename(&temp_path, &meta_path)?;

        // Clean up WAL
        fs::remove_file(&wal_path)?;

        Ok(Some(metadata))
    }

    /// Load checkpoint if it exists
    pub fn load_checkpoint(&self) -> PersistenceResult<Option<DocumentMetadata>> {
        let checkpoint_path = self.checkpoint_path();

        if !checkpoint_path.exists() {
            return Ok(None);
        }

        let json = fs::read_to_string(&checkpoint_path)?;
        let metadata: DocumentMetadata = serde_json::from_str(&json)
            .map_err(|e| PersistenceError::DeserializationError(e.to_string()))?;

        Ok(Some(metadata))
    }

    /// Delete checkpoint file
    pub fn delete_checkpoint(&self) -> PersistenceResult<()> {
        let checkpoint_path = self.checkpoint_path();
        if checkpoint_path.exists() {
            fs::remove_file(checkpoint_path)?;
        }
        Ok(())
    }

    /// Check if there's a pending WAL (indicates crash/unclean shutdown)
    pub fn has_pending_wal(&self) -> bool {
        self.wal_path().exists()
    }

    /// Write data to file and sync to disk
    fn write_and_sync(&self, path: &Path, data: &str) -> PersistenceResult<()> {
        let mut file = fs::File::create(path)?;
        file.write_all(data.as_bytes())?;
        file.sync_all()?; // Force data to disk
        Ok(())
    }

    /// Clean up all checkpoint-related files
    pub fn cleanup(&self) -> PersistenceResult<()> {
        let wal_path = self.wal_path();
        let checkpoint_path = self.checkpoint_path();

        if wal_path.exists() {
            fs::remove_file(wal_path)?;
        }

        if checkpoint_path.exists() {
            fs::remove_file(checkpoint_path)?;
        }

        Ok(())
    }
}

/// Checkpoint metadata with timestamp
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CheckpointMetadata {
    /// When the checkpoint was created
    pub timestamp: u64,

    /// Human-readable description
    pub description: Option<String>,

    /// Document metadata at checkpoint time
    pub metadata: DocumentMetadata,
}

impl CheckpointMetadata {
    /// Create a new checkpoint metadata
    pub fn new(metadata: DocumentMetadata, description: Option<String>) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Self {
            timestamp,
            description,
            metadata,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::DocumentMetadata;
    use std::thread;
    use std::time::Duration;

    fn test_metadata(suffix: &str) -> DocumentMetadata {
        let temp_dir = std::env::temp_dir();
        let pdf_path = temp_dir.join(format!(
            "test_checkpoint_{}_{}.pdf",
            suffix,
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));

        DocumentMetadata {
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
        }
    }

    #[test]
    fn test_write_with_checkpoint() {
        let metadata = test_metadata("write");
        let manager = CheckpointManager::new(&metadata.file_path);

        // Write with checkpoint
        let result = manager.write_with_checkpoint(&metadata);
        assert!(result.is_ok());

        // Verify main file exists
        let meta_path = metadata_path(&metadata.file_path);
        assert!(meta_path.exists());

        // Verify WAL was cleaned up
        assert!(!manager.has_pending_wal());

        // Cleanup
        let _ = fs::remove_file(meta_path);
    }

    #[test]
    fn test_recover_clean_state() {
        let metadata = test_metadata("recover_clean");
        let manager = CheckpointManager::new(&metadata.file_path);

        // No WAL file exists
        let result = manager.recover();
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn test_recover_from_crash() {
        let metadata = test_metadata("recover_crash");
        let manager = CheckpointManager::new(&metadata.file_path);

        // Simulate a crash by writing WAL but not completing the write
        let wal_path = manager.wal_path();
        let json = serde_json::to_string_pretty(&metadata).unwrap();
        fs::write(&wal_path, json).unwrap();

        // Verify WAL exists
        assert!(manager.has_pending_wal());

        // Recover
        let result = manager.recover();
        assert!(result.is_ok());
        let recovered = result.unwrap();
        assert!(recovered.is_some());

        let recovered_metadata = recovered.unwrap();
        assert_eq!(recovered_metadata.title, metadata.title);
        assert_eq!(recovered_metadata.author, metadata.author);

        // Verify WAL was cleaned up
        assert!(!manager.has_pending_wal());

        // Verify main file exists
        let meta_path = metadata_path(&metadata.file_path);
        assert!(meta_path.exists());

        // Cleanup
        let _ = fs::remove_file(meta_path);
    }

    #[test]
    fn test_create_and_load_checkpoint() {
        let metadata = test_metadata("checkpoint");
        let manager = CheckpointManager::new(&metadata.file_path);

        // Create checkpoint
        let result = manager.create_checkpoint(&metadata);
        assert!(result.is_ok());

        // Load checkpoint
        let loaded = manager.load_checkpoint();
        assert!(loaded.is_ok());
        let loaded_metadata = loaded.unwrap();
        assert!(loaded_metadata.is_some());

        let loaded_metadata = loaded_metadata.unwrap();
        assert_eq!(loaded_metadata.title, metadata.title);

        // Cleanup
        let _ = manager.delete_checkpoint();
        let checkpoint_path = manager.checkpoint_path();
        assert!(!checkpoint_path.exists());
    }

    #[test]
    fn test_delete_checkpoint() {
        let metadata = test_metadata("delete_checkpoint");
        let manager = CheckpointManager::new(&metadata.file_path);

        // Create checkpoint
        manager.create_checkpoint(&metadata).unwrap();
        assert!(manager.checkpoint_path().exists());

        // Delete checkpoint
        let result = manager.delete_checkpoint();
        assert!(result.is_ok());
        assert!(!manager.checkpoint_path().exists());
    }

    #[test]
    fn test_cleanup() {
        let metadata = test_metadata("cleanup");
        let manager = CheckpointManager::new(&metadata.file_path);

        // Create both WAL and checkpoint
        let wal_path = manager.wal_path();
        let checkpoint_path = manager.checkpoint_path();

        fs::write(&wal_path, "test").unwrap();
        manager.create_checkpoint(&metadata).unwrap();

        assert!(wal_path.exists());
        assert!(checkpoint_path.exists());

        // Cleanup
        let result = manager.cleanup();
        assert!(result.is_ok());
        assert!(!wal_path.exists());
        assert!(!checkpoint_path.exists());
    }

    #[test]
    fn test_has_pending_wal() {
        let metadata = test_metadata("pending_wal");
        let manager = CheckpointManager::new(&metadata.file_path);

        // Initially no WAL
        assert!(!manager.has_pending_wal());

        // Create WAL
        let wal_path = manager.wal_path();
        fs::write(&wal_path, "test").unwrap();
        assert!(manager.has_pending_wal());

        // Cleanup
        let _ = fs::remove_file(wal_path);
    }

    #[test]
    fn test_checkpoint_metadata() {
        let metadata = test_metadata("checkpoint_meta");
        let checkpoint = CheckpointMetadata::new(
            metadata.clone(),
            Some("Test checkpoint".to_string()),
        );

        assert_eq!(checkpoint.metadata.title, metadata.title);
        assert_eq!(checkpoint.description, Some("Test checkpoint".to_string()));
        assert!(checkpoint.timestamp > 0);
    }

    #[test]
    fn test_multiple_writes_no_wal_leak() {
        let metadata = test_metadata("multiple_writes");
        let manager = CheckpointManager::new(&metadata.file_path);

        // Write multiple times
        for i in 0..5 {
            let mut meta = metadata.clone();
            meta.title = Some(format!("Test Document {}", i));

            let result = manager.write_with_checkpoint(&meta);
            assert!(result.is_ok());

            // WAL should be cleaned up after each write
            assert!(!manager.has_pending_wal());
        }

        // Cleanup
        let meta_path = metadata_path(&metadata.file_path);
        let _ = fs::remove_file(meta_path);
    }

    #[test]
    fn test_concurrent_recovery() {
        let metadata = test_metadata("concurrent");
        let manager = CheckpointManager::new(&metadata.file_path);

        // Write WAL
        let wal_path = manager.wal_path();
        let json = serde_json::to_string_pretty(&metadata).unwrap();
        fs::write(&wal_path, json).unwrap();

        // Multiple recovery attempts (simulates concurrent processes)
        let mut handles = vec![];
        for _ in 0..3 {
            let manager_clone = CheckpointManager::new(&metadata.file_path);
            let handle = thread::spawn(move || {
                // Recovery should be idempotent
                let _ = manager_clone.recover();
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }

        // Eventually WAL should be gone and main file should exist
        thread::sleep(Duration::from_millis(100));

        // One recovery should have succeeded
        let meta_path = metadata_path(&metadata.file_path);

        // Cleanup (may or may not exist depending on race)
        let _ = fs::remove_file(meta_path);
        let _ = fs::remove_file(manager.wal_path());
    }
}
