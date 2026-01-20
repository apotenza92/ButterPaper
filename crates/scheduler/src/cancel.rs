//! Cancellation token system for jobs
//!
//! Provides cancellation tokens that allow running jobs to be cancelled
//! cooperatively. Workers can check if a job has been cancelled and stop
//! processing early.

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

/// Cancellation token for cooperative job cancellation
///
/// Workers can periodically check `is_cancelled()` to determine if they
/// should stop processing. Multiple tokens can share the same underlying
/// cancellation state via Arc.
///
/// # Example
///
/// ```
/// use pdf_editor_scheduler::CancellationToken;
///
/// let token = CancellationToken::new();
/// let worker_token = token.clone();
///
/// // In worker thread:
/// // while processing {
/// //     if worker_token.is_cancelled() {
/// //         return; // Stop early
/// //     }
/// //     // ... do work ...
/// // }
///
/// // In main thread:
/// token.cancel();
/// ```
#[derive(Clone)]
pub struct CancellationToken {
    cancelled: Arc<AtomicBool>,
}

impl CancellationToken {
    /// Create a new cancellation token
    ///
    /// The token starts in a non-cancelled state.
    pub fn new() -> Self {
        Self {
            cancelled: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Cancel this token
    ///
    /// All clones of this token will also observe the cancellation.
    /// This operation is idempotent - calling it multiple times is safe.
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
    }

    /// Check if this token has been cancelled
    ///
    /// Returns `true` if `cancel()` has been called on this token or any clone.
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Acquire)
    }

    /// Reset this token to non-cancelled state
    ///
    /// This allows the token to be reused. Note that all clones will also
    /// be reset.
    pub fn reset(&self) {
        self.cancelled.store(false, Ordering::Release);
    }
}

impl Default for CancellationToken {
    fn default() -> Self {
        Self::new()
    }
}

/// Cancellation token registry for tracking active jobs
///
/// Associates job IDs with cancellation tokens, allowing jobs to be cancelled
/// by ID. The scheduler uses this to provide cancellation capabilities.
///
/// # Example
///
/// ```
/// use pdf_editor_scheduler::{CancellationRegistry, JobId};
///
/// let registry = CancellationRegistry::new();
///
/// // Register a job with its token
/// let job_id: JobId = 1;
/// let token = registry.register(job_id);
///
/// // Later, cancel the job
/// registry.cancel(job_id);
///
/// // Worker can check the token
/// assert!(token.is_cancelled());
/// ```
pub struct CancellationRegistry {
    tokens: Arc<std::sync::Mutex<std::collections::HashMap<crate::JobId, CancellationToken>>>,
}

impl CancellationRegistry {
    /// Create a new empty cancellation registry
    pub fn new() -> Self {
        Self {
            tokens: Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())),
        }
    }

    /// Register a job and return its cancellation token
    ///
    /// Creates a new cancellation token for the job and stores it in the registry.
    /// Returns a clone of the token that can be given to the worker.
    pub fn register(&self, job_id: crate::JobId) -> CancellationToken {
        let token = CancellationToken::new();
        let mut tokens = self.tokens.lock().unwrap();
        tokens.insert(job_id, token.clone());
        token
    }

    /// Cancel a job by ID
    ///
    /// Cancels the token associated with the job. If the job is not found,
    /// this is a no-op. Returns `true` if the job was found and cancelled.
    pub fn cancel(&self, job_id: crate::JobId) -> bool {
        let tokens = self.tokens.lock().unwrap();
        if let Some(token) = tokens.get(&job_id) {
            token.cancel();
            true
        } else {
            false
        }
    }

    /// Cancel multiple jobs by ID
    ///
    /// Returns the number of jobs that were found and cancelled.
    pub fn cancel_many(&self, job_ids: &[crate::JobId]) -> usize {
        let mut cancelled = 0;
        let tokens = self.tokens.lock().unwrap();
        for job_id in job_ids {
            if let Some(token) = tokens.get(job_id) {
                token.cancel();
                cancelled += 1;
            }
        }
        cancelled
    }

    /// Cancel all registered jobs
    ///
    /// Returns the number of jobs cancelled.
    pub fn cancel_all(&self) -> usize {
        let tokens = self.tokens.lock().unwrap();
        let count = tokens.len();
        for token in tokens.values() {
            token.cancel();
        }
        count
    }

    /// Unregister a job (called when job completes or is removed from queue)
    ///
    /// Removes the job from the registry. Returns `true` if the job was found.
    pub fn unregister(&self, job_id: crate::JobId) -> bool {
        let mut tokens = self.tokens.lock().unwrap();
        tokens.remove(&job_id).is_some()
    }

    /// Get the cancellation token for a job
    ///
    /// Returns `None` if the job is not registered.
    pub fn get(&self, job_id: crate::JobId) -> Option<CancellationToken> {
        let tokens = self.tokens.lock().unwrap();
        tokens.get(&job_id).cloned()
    }

    /// Get the number of registered jobs
    pub fn len(&self) -> usize {
        let tokens = self.tokens.lock().unwrap();
        tokens.len()
    }

    /// Check if the registry is empty
    pub fn is_empty(&self) -> bool {
        let tokens = self.tokens.lock().unwrap();
        tokens.is_empty()
    }

    /// Clear all registered tokens
    ///
    /// Removes all tokens from the registry without cancelling them.
    pub fn clear(&self) {
        let mut tokens = self.tokens.lock().unwrap();
        tokens.clear();
    }
}

impl Default for CancellationRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cancellation_token_basic() {
        let token = CancellationToken::new();
        assert!(!token.is_cancelled());

        token.cancel();
        assert!(token.is_cancelled());
    }

    #[test]
    fn test_cancellation_token_clone() {
        let token1 = CancellationToken::new();
        let token2 = token1.clone();

        assert!(!token1.is_cancelled());
        assert!(!token2.is_cancelled());

        token1.cancel();
        assert!(token1.is_cancelled());
        assert!(token2.is_cancelled());
    }

    #[test]
    fn test_cancellation_token_idempotent() {
        let token = CancellationToken::new();

        token.cancel();
        assert!(token.is_cancelled());

        token.cancel();
        assert!(token.is_cancelled());
    }

    #[test]
    fn test_cancellation_token_reset() {
        let token = CancellationToken::new();

        token.cancel();
        assert!(token.is_cancelled());

        token.reset();
        assert!(!token.is_cancelled());
    }

    #[test]
    fn test_cancellation_token_default() {
        let token = CancellationToken::default();
        assert!(!token.is_cancelled());
    }

    #[test]
    fn test_registry_basic() {
        let registry = CancellationRegistry::new();

        let job_id = 1;
        let token = registry.register(job_id);

        assert!(!token.is_cancelled());
        assert_eq!(registry.len(), 1);

        registry.cancel(job_id);
        assert!(token.is_cancelled());
    }

    #[test]
    fn test_registry_cancel_not_found() {
        let registry = CancellationRegistry::new();

        let cancelled = registry.cancel(999);
        assert!(!cancelled);
    }

    #[test]
    fn test_registry_cancel_many() {
        let registry = CancellationRegistry::new();

        let token1 = registry.register(1);
        let token2 = registry.register(2);
        let token3 = registry.register(3);

        let cancelled = registry.cancel_many(&[1, 2, 999]);
        assert_eq!(cancelled, 2);

        assert!(token1.is_cancelled());
        assert!(token2.is_cancelled());
        assert!(!token3.is_cancelled());
    }

    #[test]
    fn test_registry_cancel_all() {
        let registry = CancellationRegistry::new();

        let token1 = registry.register(1);
        let token2 = registry.register(2);
        let token3 = registry.register(3);

        let cancelled = registry.cancel_all();
        assert_eq!(cancelled, 3);

        assert!(token1.is_cancelled());
        assert!(token2.is_cancelled());
        assert!(token3.is_cancelled());
    }

    #[test]
    fn test_registry_unregister() {
        let registry = CancellationRegistry::new();

        let job_id = 1;
        registry.register(job_id);
        assert_eq!(registry.len(), 1);

        let removed = registry.unregister(job_id);
        assert!(removed);
        assert_eq!(registry.len(), 0);

        let removed_again = registry.unregister(job_id);
        assert!(!removed_again);
    }

    #[test]
    fn test_registry_get() {
        let registry = CancellationRegistry::new();

        let job_id = 1;
        let token1 = registry.register(job_id);

        let token2 = registry.get(job_id).unwrap();
        assert!(!token2.is_cancelled());

        token1.cancel();
        assert!(token2.is_cancelled());

        let token3 = registry.get(999);
        assert!(token3.is_none());
    }

    #[test]
    fn test_registry_clear() {
        let registry = CancellationRegistry::new();

        let token1 = registry.register(1);
        let token2 = registry.register(2);

        assert_eq!(registry.len(), 2);

        registry.clear();
        assert_eq!(registry.len(), 0);

        // Tokens should still be valid, just not in registry
        assert!(!token1.is_cancelled());
        assert!(!token2.is_cancelled());
    }

    #[test]
    fn test_registry_is_empty() {
        let registry = CancellationRegistry::new();
        assert!(registry.is_empty());

        registry.register(1);
        assert!(!registry.is_empty());

        registry.clear();
        assert!(registry.is_empty());
    }

    #[test]
    fn test_registry_default() {
        let registry = CancellationRegistry::default();
        assert!(registry.is_empty());
    }
}
