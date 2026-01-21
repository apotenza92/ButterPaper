//! Write coordinator for batched atomic writes
//!
//! Manages in-memory working state with batched atomic writes to disk.
//! Provides debouncing to avoid frequent I/O operations while ensuring
//! eventual persistence of all changes.

use crate::checkpoint::CheckpointManager;
use crate::document::DocumentMetadata;
use crate::persistence::{save_metadata, PersistenceResult};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

/// Configuration for the write coordinator
#[derive(Debug, Clone)]
pub struct WriteCoordinatorConfig {
    /// Minimum interval between writes (debounce duration)
    pub debounce_duration: Duration,

    /// Maximum interval before forcing a write (even if still receiving changes)
    pub max_debounce_duration: Duration,

    /// Whether to enable automatic batched writes (if false, only manual saves)
    pub enable_auto_save: bool,

    /// Whether to use crash-safe checkpoints (WAL pattern)
    pub enable_checkpoints: bool,
}

impl Default for WriteCoordinatorConfig {
    fn default() -> Self {
        Self {
            debounce_duration: Duration::from_secs(2),
            max_debounce_duration: Duration::from_secs(10),
            enable_auto_save: true,
            enable_checkpoints: true,
        }
    }
}

/// State tracking for pending writes
#[derive(Debug)]
struct PendingWrite {
    /// When the first change was marked
    first_marked_at: Instant,

    /// When the most recent change was marked
    last_marked_at: Instant,

    /// Whether there are pending changes
    is_dirty: bool,
}

impl PendingWrite {
    fn new() -> Self {
        Self {
            first_marked_at: Instant::now(),
            last_marked_at: Instant::now(),
            is_dirty: false,
        }
    }

    fn mark_dirty(&mut self) {
        let now = Instant::now();
        if !self.is_dirty {
            self.first_marked_at = now;
        }
        self.last_marked_at = now;
        self.is_dirty = true;
    }

    fn clear(&mut self) {
        self.is_dirty = false;
    }

    fn should_write(&self, config: &WriteCoordinatorConfig) -> bool {
        if !self.is_dirty {
            return false;
        }

        let elapsed_since_last = self.last_marked_at.elapsed();
        let elapsed_since_first = self.first_marked_at.elapsed();

        // Write if debounce duration has passed since last change
        // OR if max debounce duration has passed since first change
        elapsed_since_last >= config.debounce_duration
            || elapsed_since_first >= config.max_debounce_duration
    }
}

/// Write coordinator that manages batched atomic writes
///
/// Keeps working state in memory and batches writes to avoid frequent I/O.
/// Uses atomic writes (temp file + rename) to ensure consistency.
pub struct WriteCoordinator {
    /// Configuration
    config: WriteCoordinatorConfig,

    /// Pending write state
    pending: Arc<Mutex<PendingWrite>>,

    /// Metadata to write (protected by mutex for thread-safe access)
    metadata: Arc<Mutex<Option<DocumentMetadata>>>,

    /// Background thread handle
    _thread_handle: Option<thread::JoinHandle<()>>,

    /// Flag to stop background thread
    should_stop: Arc<Mutex<bool>>,
}

impl WriteCoordinator {
    /// Create a new write coordinator with default configuration
    pub fn new() -> Self {
        Self::with_config(WriteCoordinatorConfig::default())
    }

    /// Create a new write coordinator with custom configuration
    pub fn with_config(config: WriteCoordinatorConfig) -> Self {
        let pending = Arc::new(Mutex::new(PendingWrite::new()));
        let metadata = Arc::new(Mutex::new(None));
        let should_stop = Arc::new(Mutex::new(false));

        let thread_handle = if config.enable_auto_save {
            Some(Self::spawn_background_thread(
                Arc::clone(&pending),
                Arc::clone(&metadata),
                Arc::clone(&should_stop),
                config.clone(),
            ))
        } else {
            None
        };

        Self {
            config,
            pending,
            metadata,
            _thread_handle: thread_handle,
            should_stop,
        }
    }

    /// Mark metadata as dirty (needs to be written)
    ///
    /// This should be called whenever document metadata changes.
    /// The write will be batched and happen after the debounce duration.
    pub fn mark_dirty(&self, metadata: DocumentMetadata) {
        *self.metadata.lock().unwrap() = Some(metadata);
        self.pending.lock().unwrap().mark_dirty();
    }

    /// Check if there are pending writes
    pub fn is_dirty(&self) -> bool {
        self.pending.lock().unwrap().is_dirty
    }

    /// Force an immediate write of pending changes
    ///
    /// Returns Ok(true) if a write was performed, Ok(false) if nothing to write,
    /// or Err if the write failed.
    pub fn flush(&self) -> PersistenceResult<bool> {
        let mut pending = self.pending.lock().unwrap();

        if !pending.is_dirty {
            return Ok(false);
        }

        let metadata = self.metadata.lock().unwrap();
        if let Some(ref meta) = *metadata {
            if self.config.enable_checkpoints {
                // Use checkpoint manager for crash-safe writes
                let checkpoint_mgr = CheckpointManager::new(&meta.file_path);
                checkpoint_mgr.write_with_checkpoint(meta)?;
            } else {
                // Use standard save
                save_metadata(meta)?;
            }
            drop(metadata); // Release lock before clearing dirty flag
            pending.clear();
            Ok(true)
        } else {
            // No metadata to write
            pending.clear();
            Ok(false)
        }
    }

    /// Recover from a crash by replaying any pending WAL
    ///
    /// Should be called on startup before loading metadata normally.
    /// Returns Ok(Some(metadata)) if recovery was needed and successful.
    pub fn recover(pdf_path: impl AsRef<std::path::Path>) -> PersistenceResult<Option<DocumentMetadata>> {
        let checkpoint_mgr = CheckpointManager::new(pdf_path);
        checkpoint_mgr.recover()
    }

    /// Get the current configuration
    pub fn config(&self) -> &WriteCoordinatorConfig {
        &self.config
    }

    /// Spawn background thread for periodic writes
    fn spawn_background_thread(
        pending: Arc<Mutex<PendingWrite>>,
        metadata: Arc<Mutex<Option<DocumentMetadata>>>,
        should_stop: Arc<Mutex<bool>>,
        config: WriteCoordinatorConfig,
    ) -> thread::JoinHandle<()> {
        thread::spawn(move || {
            // Check every 500ms for pending writes
            let check_interval = Duration::from_millis(500);

            loop {
                // Check if we should stop
                if *should_stop.lock().unwrap() {
                    break;
                }

                // Check if there's work to do
                let should_write = {
                    let pending = pending.lock().unwrap();
                    pending.should_write(&config)
                };

                if should_write {
                    // Perform the write
                    let mut pending = pending.lock().unwrap();
                    if pending.is_dirty {
                        let metadata = metadata.lock().unwrap();
                        if let Some(ref meta) = *metadata {
                            // Attempt to save (ignore errors in background thread)
                            if config.enable_checkpoints {
                                let checkpoint_mgr = CheckpointManager::new(&meta.file_path);
                                let _ = checkpoint_mgr.write_with_checkpoint(meta);
                            } else {
                                let _ = save_metadata(meta);
                            }
                        }
                        drop(metadata);
                        pending.clear();
                    }
                }

                thread::sleep(check_interval);
            }
        })
    }
}

impl Default for WriteCoordinator {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for WriteCoordinator {
    fn drop(&mut self) {
        // Signal background thread to stop
        *self.should_stop.lock().unwrap() = true;

        // Flush any pending writes before dropping
        let _ = self.flush();
    }
}

#[cfg(test)]
impl WriteCoordinator {
    /// Helper method for testing (allow cloning internal state)
    fn clone_for_test(&self) -> Self {
        Self {
            config: self.config.clone(),
            pending: Arc::clone(&self.pending),
            metadata: Arc::clone(&self.metadata),
            _thread_handle: None,
            should_stop: Arc::clone(&self.should_stop),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::DocumentMetadata;
    use std::thread;
    use std::time::Duration;

    fn test_metadata() -> DocumentMetadata {
        let temp_dir = std::env::temp_dir();
        let pdf_path = temp_dir.join(format!("test_write_coordinator_{}.pdf",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()));

        DocumentMetadata {
            title: Some("Test Document".to_string()),
            author: Some("Test Author".to_string()),
            subject: None,
            creator: None,
            producer: None,
            page_count: 5,
            file_path: pdf_path,
            file_size: 1024,
            page_dimensions: std::collections::HashMap::new(),
            scale_systems: Vec::new(),
            default_scales: std::collections::HashMap::new(),
            text_edits: Vec::new(),
            annotations: Vec::new(),
        }
    }

    #[test]
    fn test_write_coordinator_creation() {
        let coordinator = WriteCoordinator::new();
        assert!(!coordinator.is_dirty());
    }

    #[test]
    fn test_mark_dirty() {
        let coordinator = WriteCoordinator::with_config(WriteCoordinatorConfig {
            enable_auto_save: false,
            ..Default::default()
        });

        let metadata = test_metadata();
        assert!(!coordinator.is_dirty());

        coordinator.mark_dirty(metadata);
        assert!(coordinator.is_dirty());
    }

    #[test]
    fn test_flush_when_clean() {
        let coordinator = WriteCoordinator::with_config(WriteCoordinatorConfig {
            enable_auto_save: false,
            ..Default::default()
        });

        let result = coordinator.flush();
        assert!(result.is_ok());
        assert!(!result.unwrap()); // Nothing to write
    }

    #[test]
    fn test_flush_when_dirty() {
        let coordinator = WriteCoordinator::with_config(WriteCoordinatorConfig {
            enable_auto_save: false,
            ..Default::default()
        });

        let metadata = test_metadata();
        coordinator.mark_dirty(metadata.clone());
        assert!(coordinator.is_dirty());

        let result = coordinator.flush();
        assert!(result.is_ok());
        assert!(result.unwrap()); // Write was performed
        assert!(!coordinator.is_dirty());

        // Verify file was written
        assert!(crate::persistence::metadata_exists(&metadata.file_path));

        // Cleanup
        let _ = crate::persistence::delete_metadata(&metadata.file_path);
    }

    #[test]
    fn test_auto_save_disabled() {
        let coordinator = WriteCoordinator::with_config(WriteCoordinatorConfig {
            enable_auto_save: false,
            enable_checkpoints: false,
            debounce_duration: Duration::from_millis(100),
            max_debounce_duration: Duration::from_millis(200),
        });

        let metadata = test_metadata();
        coordinator.mark_dirty(metadata.clone());
        assert!(coordinator.is_dirty());

        // Wait for longer than debounce duration
        thread::sleep(Duration::from_millis(300));

        // Should still be dirty (auto-save disabled)
        assert!(coordinator.is_dirty());

        // Cleanup
        let _ = crate::persistence::delete_metadata(&metadata.file_path);
    }

    #[test]
    fn test_auto_save_with_debounce() {
        let coordinator = WriteCoordinator::with_config(WriteCoordinatorConfig {
            enable_auto_save: true,
            enable_checkpoints: false,
            debounce_duration: Duration::from_millis(500),
            max_debounce_duration: Duration::from_millis(1000),
        });

        let metadata = test_metadata();
        coordinator.mark_dirty(metadata.clone());
        assert!(coordinator.is_dirty());

        // Wait for debounce + processing time
        thread::sleep(Duration::from_millis(800));

        // Should have been auto-saved
        assert!(!coordinator.is_dirty());
        assert!(crate::persistence::metadata_exists(&metadata.file_path));

        // Cleanup
        let _ = crate::persistence::delete_metadata(&metadata.file_path);
    }

    #[test]
    fn test_multiple_changes_batched() {
        let coordinator = WriteCoordinator::with_config(WriteCoordinatorConfig {
            enable_auto_save: true,
            enable_checkpoints: false,
            debounce_duration: Duration::from_millis(500),
            max_debounce_duration: Duration::from_millis(1000),
        });

        let mut metadata = test_metadata();

        // Make multiple rapid changes
        for i in 0..5 {
            metadata.title = Some(format!("Test Document {}", i));
            coordinator.mark_dirty(metadata.clone());
            thread::sleep(Duration::from_millis(50));
        }

        // Wait for debounce + check interval + buffer
        // (250ms for changes + 500ms debounce + 500ms check interval + 200ms buffer)
        thread::sleep(Duration::from_millis(1500));

        // Should have been saved once with final state
        assert!(!coordinator.is_dirty());
        assert!(crate::persistence::metadata_exists(&metadata.file_path));

        let loaded = crate::persistence::load_metadata(&metadata.file_path)
            .unwrap()
            .unwrap();
        assert_eq!(loaded.title, Some("Test Document 4".to_string()));

        // Cleanup
        let _ = crate::persistence::delete_metadata(&metadata.file_path);
    }

    #[test]
    fn test_max_debounce_forces_write() {
        let coordinator = WriteCoordinator::with_config(WriteCoordinatorConfig {
            enable_auto_save: true,
            enable_checkpoints: false,
            debounce_duration: Duration::from_secs(10), // Long debounce
            max_debounce_duration: Duration::from_millis(500), // Short max
        });

        let metadata = test_metadata();
        coordinator.mark_dirty(metadata.clone());

        // Keep marking dirty (simulating continuous changes)
        let coordinator_clone = coordinator.clone_for_test();
        let metadata_clone = metadata.clone();
        let handle = thread::spawn(move || {
            for _ in 0..10 {
                let mut meta = metadata_clone.clone();
                meta.title = Some(format!("Updated {}", std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis()));
                coordinator_clone.mark_dirty(meta);
                thread::sleep(Duration::from_millis(50));
            }
        });

        handle.join().unwrap();

        // Wait for max debounce + processing
        thread::sleep(Duration::from_millis(800));

        // Should have been saved despite continuous changes
        assert!(!coordinator.is_dirty());
        assert!(crate::persistence::metadata_exists(&metadata.file_path));

        // Cleanup
        let _ = crate::persistence::delete_metadata(&metadata.file_path);
    }

    #[test]
    fn test_pending_write_should_write_logic() {
        let config = WriteCoordinatorConfig {
            debounce_duration: Duration::from_millis(100),
            max_debounce_duration: Duration::from_millis(300),
            enable_auto_save: true,
            enable_checkpoints: false,
        };

        let mut pending = PendingWrite::new();

        // Not dirty yet
        assert!(!pending.should_write(&config));

        // Mark dirty
        pending.mark_dirty();
        assert!(pending.is_dirty);

        // Too soon
        assert!(!pending.should_write(&config));

        // Wait for debounce
        thread::sleep(Duration::from_millis(150));
        assert!(pending.should_write(&config));

        // Clear and mark again
        pending.clear();
        assert!(!pending.should_write(&config));

        pending.mark_dirty();
        thread::sleep(Duration::from_millis(50));

        // Mark again (resets debounce)
        pending.mark_dirty();
        assert!(!pending.should_write(&config));

        // Wait for max debounce
        thread::sleep(Duration::from_millis(300));
        assert!(pending.should_write(&config));
    }

    #[test]
    fn test_checkpoint_enabled() {
        let coordinator = WriteCoordinator::with_config(WriteCoordinatorConfig {
            enable_auto_save: false,
            enable_checkpoints: true,
            ..Default::default()
        });

        let metadata = test_metadata();
        coordinator.mark_dirty(metadata.clone());

        let result = coordinator.flush();
        assert!(result.is_ok());
        assert!(result.unwrap());

        // Verify file was written
        assert!(crate::persistence::metadata_exists(&metadata.file_path));

        // Verify no WAL file left behind
        let checkpoint_mgr = CheckpointManager::new(&metadata.file_path);
        assert!(!checkpoint_mgr.has_pending_wal());

        // Cleanup
        let _ = crate::persistence::delete_metadata(&metadata.file_path);
    }

    #[test]
    fn test_recovery_integration() {
        let metadata = test_metadata();
        let pdf_path = metadata.file_path.clone();

        // Simulate crash by creating WAL without completing write
        let checkpoint_mgr = CheckpointManager::new(&pdf_path);
        let json = serde_json::to_string_pretty(&metadata).unwrap();
        std::fs::write(checkpoint_mgr.wal_path(), json).unwrap();

        // Verify WAL exists
        assert!(checkpoint_mgr.has_pending_wal());

        // Recover using WriteCoordinator
        let recovered = WriteCoordinator::recover(&pdf_path);
        assert!(recovered.is_ok());
        let recovered_metadata = recovered.unwrap();
        assert!(recovered_metadata.is_some());

        let recovered_metadata = recovered_metadata.unwrap();
        assert_eq!(recovered_metadata.title, metadata.title);

        // Verify WAL was cleaned up
        assert!(!checkpoint_mgr.has_pending_wal());

        // Cleanup
        let _ = crate::persistence::delete_metadata(&pdf_path);
    }

    #[test]
    fn test_checkpoint_disabled() {
        let coordinator = WriteCoordinator::with_config(WriteCoordinatorConfig {
            enable_auto_save: false,
            enable_checkpoints: false,
            ..Default::default()
        });

        let metadata = test_metadata();
        coordinator.mark_dirty(metadata.clone());

        let result = coordinator.flush();
        assert!(result.is_ok());
        assert!(result.unwrap());

        // Verify file was written
        assert!(crate::persistence::metadata_exists(&metadata.file_path));

        // Cleanup
        let _ = crate::persistence::delete_metadata(&metadata.file_path);
    }

    #[test]
    fn test_auto_save_with_checkpoints() {
        let coordinator = WriteCoordinator::with_config(WriteCoordinatorConfig {
            enable_auto_save: true,
            enable_checkpoints: true,
            debounce_duration: Duration::from_millis(500),
            max_debounce_duration: Duration::from_millis(1000),
        });

        let metadata = test_metadata();
        coordinator.mark_dirty(metadata.clone());
        assert!(coordinator.is_dirty());

        // Wait for auto-save
        thread::sleep(Duration::from_millis(800));

        // Should have been saved
        assert!(!coordinator.is_dirty());
        assert!(crate::persistence::metadata_exists(&metadata.file_path));

        // No WAL should be left behind
        let checkpoint_mgr = CheckpointManager::new(&metadata.file_path);
        assert!(!checkpoint_mgr.has_pending_wal());

        // Cleanup
        let _ = crate::persistence::delete_metadata(&metadata.file_path);
    }
}
