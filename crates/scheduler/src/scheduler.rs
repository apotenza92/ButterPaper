//! Job scheduler implementation
//!
//! Provides a high-level job scheduler that manages job submission,
//! priority-based execution ordering, and job lifecycle.

use crate::cancel::{CancellationRegistry, CancellationToken};
use crate::priority::{Job, JobId, JobPriority, JobType, PriorityQueue};
use crate::viewport::{PriorityCalculator, TilePosition, Viewport};
use std::sync::{Arc, Mutex};

/// Job scheduler statistics
#[derive(Debug, Clone, Default)]
pub struct SchedulerStats {
    /// Total jobs submitted
    pub jobs_submitted: u64,

    /// Total jobs completed
    pub jobs_completed: u64,

    /// Total jobs cancelled
    pub jobs_cancelled: u64,

    /// Current queue size
    pub queue_size: usize,
}

impl SchedulerStats {
    /// Get the number of jobs currently pending
    pub fn pending_jobs(&self) -> u64 {
        self.jobs_submitted - self.jobs_completed - self.jobs_cancelled
    }
}

/// Job scheduler with priority queue
///
/// Thread-safe scheduler that manages job submission and execution ordering.
/// Jobs are executed in priority order, with higher priority jobs running first.
/// Supports cancellation tokens for cooperative job cancellation.
///
/// # Example
///
/// ```
/// use pdf_editor_scheduler::{JobScheduler, JobPriority, JobType};
/// use std::path::PathBuf;
///
/// let scheduler = JobScheduler::new();
///
/// // Submit a high-priority job
/// let (job_id, token) = scheduler.submit(JobPriority::Visible, JobType::LoadFile {
///     path: PathBuf::from("document.pdf")
/// });
///
/// // Get the next job to execute
/// if let Some(job) = scheduler.next_job() {
///     println!("Executing job {}: {:?}", job.id, job.job_type);
///     // Worker can check token.is_cancelled() during execution
///     scheduler.complete_job(job.id);
/// }
/// ```
pub struct JobScheduler {
    queue: PriorityQueue,
    state: Arc<Mutex<SchedulerState>>,
    cancellation: CancellationRegistry,
}

struct SchedulerState {
    stats: SchedulerStats,
}

impl JobScheduler {
    /// Create a new job scheduler
    pub fn new() -> Self {
        Self {
            queue: PriorityQueue::new(),
            state: Arc::new(Mutex::new(SchedulerState {
                stats: SchedulerStats::default(),
            })),
            cancellation: CancellationRegistry::new(),
        }
    }

    /// Submit a job to the scheduler
    ///
    /// The job will be queued according to its priority and executed when
    /// a worker becomes available. A cancellation token is created for the job
    /// and registered in the cancellation registry.
    ///
    /// Returns a tuple of (job_id, cancellation_token).
    pub fn submit(&self, priority: JobPriority, job_type: JobType) -> (JobId, CancellationToken) {
        let job_id = self.queue.push(priority, job_type);
        let token = self.cancellation.register(job_id);

        let mut state = self.state.lock().unwrap();
        state.stats.jobs_submitted += 1;

        (job_id, token)
    }

    /// Get the next job to execute
    ///
    /// Returns the highest priority job from the queue, or `None` if the queue is empty.
    /// The job is removed from the queue but its cancellation token remains registered
    /// until `complete_job()` or `cancel_job()` is called.
    pub fn next_job(&self) -> Option<Job> {
        self.queue.pop()
    }

    /// Mark a job as completed
    ///
    /// This updates the scheduler statistics and unregisters the cancellation token.
    pub fn complete_job(&self, job_id: JobId) {
        let mut state = self.state.lock().unwrap();
        state.stats.jobs_completed += 1;
        drop(state); // Release lock before unregistering

        self.cancellation.unregister(job_id);
    }

    /// Cancel a specific job by ID
    ///
    /// Cancels the job's cancellation token and removes it from the queue if it
    /// hasn't started executing yet. If the job is already running, the token
    /// is marked as cancelled and the worker should check it cooperatively.
    /// Returns `true` if the job was found (either in queue or running).
    pub fn cancel_job(&self, job_id: JobId) -> bool {
        // Cancel the token (works for both queued and running jobs)
        let token_cancelled = self.cancellation.cancel(job_id);

        // Try to remove from queue (only works if not yet started)
        let removed = self.queue.remove_if(|job| job.id == job_id);

        if removed > 0 {
            let mut state = self.state.lock().unwrap();
            state.stats.jobs_cancelled += removed as u64;
            drop(state);

            // Unregister since it was removed from queue
            self.cancellation.unregister(job_id);
            true
        } else {
            // Job might be running, token was cancelled
            token_cancelled
        }
    }

    /// Cancel all jobs matching a predicate
    ///
    /// Cancels tokens and removes jobs from the queue that match the predicate.
    /// Returns the number of jobs cancelled.
    pub fn cancel_jobs_if<F>(&self, predicate: F) -> usize
    where
        F: Fn(&Job) -> bool,
    {
        // Get jobs matching predicate before removing
        let jobs_to_cancel: Vec<JobId> = self
            .queue
            .jobs()
            .into_iter()
            .filter(|job| predicate(job))
            .map(|job| job.id)
            .collect();

        // Cancel their tokens
        self.cancellation.cancel_many(&jobs_to_cancel);

        // Remove from queue
        let removed = self.queue.remove_if(predicate);

        if removed > 0 {
            let mut state = self.state.lock().unwrap();
            state.stats.jobs_cancelled += removed as u64;
            drop(state);

            // Unregister cancelled jobs
            for job_id in jobs_to_cancel {
                self.cancellation.unregister(job_id);
            }
        }

        removed
    }

    /// Cancel all jobs for a specific page
    ///
    /// Useful when the user navigates away from a page.
    pub fn cancel_page_jobs(&self, page_index: u16) -> usize {
        self.cancel_jobs_if(|job| match &job.job_type {
            JobType::RenderTile { page_index: pi, .. } => *pi == page_index,
            JobType::GenerateThumbnail { page_index: pi, .. } => *pi == page_index,
            JobType::RunOcr { page_index: pi } => *pi == page_index,
            JobType::LoadFile { .. } => false,
        })
    }

    /// Cancel all jobs except those matching a predicate
    ///
    /// Useful for aggressive cancellation when changing context (e.g., switching documents).
    pub fn cancel_all_except<F>(&self, keep_predicate: F) -> usize
    where
        F: Fn(&Job) -> bool,
    {
        self.cancel_jobs_if(|job| !keep_predicate(job))
    }

    /// Get the current number of pending jobs
    pub fn pending_jobs(&self) -> usize {
        self.queue.len()
    }

    /// Check if the scheduler has any pending jobs
    pub fn has_pending_jobs(&self) -> bool {
        !self.queue.is_empty()
    }

    /// Clear all pending jobs
    ///
    /// Cancels all jobs in the queue and their cancellation tokens.
    pub fn clear(&self) {
        let cancelled = self.queue.len();

        // Cancel all tokens
        self.cancellation.cancel_all();

        // Clear queue
        self.queue.clear();

        if cancelled > 0 {
            let mut state = self.state.lock().unwrap();
            state.stats.jobs_cancelled += cancelled as u64;
            drop(state);

            // Clear cancellation registry
            self.cancellation.clear();
        }
    }

    /// Get scheduler statistics
    pub fn stats(&self) -> SchedulerStats {
        let state = self.state.lock().unwrap();
        let mut stats = state.stats.clone();
        stats.queue_size = self.queue.len();
        stats
    }

    /// Peek at the next job without removing it
    ///
    /// Useful for deciding whether to process the next job.
    pub fn peek_next_job(&self) -> Option<Job> {
        self.queue.peek()
    }

    /// Get all pending jobs (for debugging/inspection)
    ///
    /// Jobs are returned in arbitrary order (not priority order).
    pub fn pending_jobs_list(&self) -> Vec<Job> {
        self.queue.jobs()
    }

    /// Get the cancellation token for a job
    ///
    /// Returns `None` if the job is not found or has already completed.
    /// Useful for workers that need to access the token after retrieving a job.
    pub fn get_cancellation_token(&self, job_id: JobId) -> Option<CancellationToken> {
        self.cancellation.get(job_id)
    }

    /// Aggressively cancel off-screen content based on viewport
    ///
    /// Cancels all jobs that are not visible or in the margin area around the viewport.
    /// This is useful when the viewport changes (pan, zoom, or page switch) to free up
    /// resources for rendering visible content.
    ///
    /// # Arguments
    ///
    /// * `viewport` - Current viewport state
    /// * `tile_size` - Tile size in pixels (typically 256)
    ///
    /// Returns the number of jobs cancelled.
    ///
    /// # Example
    ///
    /// ```
    /// use pdf_editor_scheduler::{JobScheduler, JobPriority, JobType, Viewport};
    ///
    /// let scheduler = JobScheduler::new();
    /// let viewport = Viewport::new(0, 0.0, 0.0, 800.0, 600.0, 100);
    ///
    /// // Submit some tile render jobs
    /// scheduler.submit(JobPriority::Visible, JobType::RenderTile {
    ///     page_index: 0,
    ///     tile_x: 0,
    ///     tile_y: 0,
    ///     zoom_level: 100,
    ///     rotation: 0,
    ///     is_preview: true,
    /// });
    ///
    /// // Cancel off-screen jobs when viewport changes
    /// let cancelled = scheduler.cancel_offscreen_jobs(&viewport, 256);
    /// ```
    pub fn cancel_offscreen_jobs(&self, viewport: &Viewport, tile_size: u32) -> usize {
        let calculator = PriorityCalculator::new(viewport.clone(), tile_size);

        self.cancel_jobs_if(|job| {
            match &job.job_type {
                JobType::RenderTile {
                    page_index,
                    tile_x,
                    tile_y,
                    zoom_level,
                    ..
                } => {
                    // Create tile position
                    let tile_pos = TilePosition::new(*page_index, *tile_x, *tile_y, *zoom_level);

                    // Calculate priority for this tile
                    let priority = calculator.calculate_tile_priority(&tile_pos);

                    // Cancel if not visible or in margin (keep only Visible and Margin priority jobs)
                    priority != JobPriority::Visible && priority != JobPriority::Margin
                }
                JobType::GenerateThumbnail { page_index, .. } => {
                    // Keep thumbnails for current and adjacent pages
                    let priority = calculator.calculate_thumbnail_priority(*page_index);
                    priority != JobPriority::Margin && priority != JobPriority::Adjacent
                }
                JobType::RunOcr { .. } => {
                    // Always cancel OCR jobs during aggressive cancellation
                    true
                }
                JobType::LoadFile { .. } => {
                    // Never cancel file loading jobs
                    false
                }
            }
        })
    }

    /// Cancel jobs that are not related to the current viewport
    ///
    /// This is a more aggressive version of `cancel_offscreen_jobs` that only keeps
    /// jobs that are currently visible in the viewport. Useful for rapid viewport changes
    /// like fast scrolling or zooming.
    ///
    /// # Arguments
    ///
    /// * `viewport` - Current viewport state
    /// * `tile_size` - Tile size in pixels (typically 256)
    ///
    /// Returns the number of jobs cancelled.
    pub fn cancel_all_except_visible(&self, viewport: &Viewport, tile_size: u32) -> usize {
        let calculator = PriorityCalculator::new(viewport.clone(), tile_size);

        self.cancel_jobs_if(|job| {
            match &job.job_type {
                JobType::RenderTile {
                    page_index,
                    tile_x,
                    tile_y,
                    zoom_level,
                    ..
                } => {
                    // Create tile position
                    let tile_pos = TilePosition::new(*page_index, *tile_x, *tile_y, *zoom_level);

                    // Calculate priority for this tile
                    let priority = calculator.calculate_tile_priority(&tile_pos);

                    // Cancel if not visible (keep only Visible priority jobs)
                    priority != JobPriority::Visible
                }
                JobType::GenerateThumbnail { .. } => {
                    // Cancel all thumbnail jobs during aggressive cancellation
                    true
                }
                JobType::RunOcr { .. } => {
                    // Cancel all OCR jobs during aggressive cancellation
                    true
                }
                JobType::LoadFile { .. } => {
                    // Never cancel file loading jobs
                    false
                }
            }
        })
    }
}

impl Default for JobScheduler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_scheduler_basic() {
        let scheduler = JobScheduler::new();

        assert_eq!(scheduler.pending_jobs(), 0);
        assert!(!scheduler.has_pending_jobs());

        let (job_id, token) = scheduler.submit(
            JobPriority::Visible,
            JobType::LoadFile {
                path: PathBuf::from("test.pdf"),
            },
        );

        assert_eq!(scheduler.pending_jobs(), 1);
        assert!(scheduler.has_pending_jobs());
        assert!(!token.is_cancelled());

        let job = scheduler.next_job().unwrap();
        assert_eq!(job.id, job_id);

        scheduler.complete_job(job_id);

        let stats = scheduler.stats();
        assert_eq!(stats.jobs_submitted, 1);
        assert_eq!(stats.jobs_completed, 1);
        assert_eq!(stats.jobs_cancelled, 0);
    }

    #[test]
    fn test_scheduler_priority_ordering() {
        let scheduler = JobScheduler::new();

        // Submit jobs in reverse priority order
        scheduler.submit(JobPriority::Ocr, JobType::RunOcr { page_index: 0 });
        scheduler.submit(
            JobPriority::Thumbnails,
            JobType::GenerateThumbnail {
                page_index: 0,
                width: 100,
                height: 100,
            },
        );
        scheduler.submit(
            JobPriority::Visible,
            JobType::LoadFile {
                path: PathBuf::from("test.pdf"),
            },
        );

        // Should get jobs in priority order
        assert_eq!(
            scheduler.next_job().unwrap().priority,
            JobPriority::Visible
        );
        assert_eq!(
            scheduler.next_job().unwrap().priority,
            JobPriority::Thumbnails
        );
        assert_eq!(scheduler.next_job().unwrap().priority, JobPriority::Ocr);
        assert!(scheduler.next_job().is_none());
    }

    #[test]
    fn test_cancel_job() {
        let scheduler = JobScheduler::new();

        let (job_id, token) = scheduler.submit(
            JobPriority::Visible,
            JobType::LoadFile {
                path: PathBuf::from("test.pdf"),
            },
        );

        assert_eq!(scheduler.pending_jobs(), 1);
        assert!(!token.is_cancelled());

        let cancelled = scheduler.cancel_job(job_id);
        assert!(cancelled);
        assert!(token.is_cancelled());

        assert_eq!(scheduler.pending_jobs(), 0);

        let stats = scheduler.stats();
        assert_eq!(stats.jobs_submitted, 1);
        assert_eq!(stats.jobs_cancelled, 1);
    }

    #[test]
    fn test_cancel_job_not_found() {
        let scheduler = JobScheduler::new();

        let cancelled = scheduler.cancel_job(999);
        assert!(!cancelled);

        let stats = scheduler.stats();
        assert_eq!(stats.jobs_cancelled, 0);
    }

    #[test]
    fn test_cancel_page_jobs() {
        let scheduler = JobScheduler::new();

        scheduler.submit(
            JobPriority::Visible,
            JobType::RenderTile {
                page_index: 0,
                tile_x: 0,
                tile_y: 0,
                zoom_level: 100,
                rotation: 0,
                is_preview: true,
            },
        );
        scheduler.submit(
            JobPriority::Visible,
            JobType::RenderTile {
                page_index: 1,
                tile_x: 0,
                tile_y: 0,
                zoom_level: 100,
                rotation: 0,
                is_preview: true,
            },
        );
        scheduler.submit(JobPriority::Ocr, JobType::RunOcr { page_index: 0 });

        assert_eq!(scheduler.pending_jobs(), 3);

        let cancelled = scheduler.cancel_page_jobs(0);
        assert_eq!(cancelled, 2);
        assert_eq!(scheduler.pending_jobs(), 1);

        // Remaining job should be for page 1
        let remaining = scheduler.next_job().unwrap();
        if let JobType::RenderTile { page_index, .. } = remaining.job_type {
            assert_eq!(page_index, 1);
        } else {
            panic!("Expected RenderTile job");
        }
    }

    #[test]
    fn test_cancel_all_except() {
        let scheduler = JobScheduler::new();

        scheduler.submit(
            JobPriority::Visible,
            JobType::LoadFile {
                path: PathBuf::from("test.pdf"),
            },
        );
        scheduler.submit(JobPriority::Ocr, JobType::RunOcr { page_index: 0 });
        scheduler.submit(JobPriority::Ocr, JobType::RunOcr { page_index: 1 });

        assert_eq!(scheduler.pending_jobs(), 3);

        // Cancel all except visible priority jobs
        let cancelled =
            scheduler.cancel_all_except(|job| job.priority == JobPriority::Visible);
        assert_eq!(cancelled, 2);
        assert_eq!(scheduler.pending_jobs(), 1);

        // Remaining job should be Visible priority
        let remaining = scheduler.next_job().unwrap();
        assert_eq!(remaining.priority, JobPriority::Visible);
    }

    #[test]
    fn test_clear() {
        let scheduler = JobScheduler::new();

        scheduler.submit(
            JobPriority::Visible,
            JobType::LoadFile {
                path: PathBuf::from("test.pdf"),
            },
        );
        scheduler.submit(JobPriority::Ocr, JobType::RunOcr { page_index: 0 });

        assert_eq!(scheduler.pending_jobs(), 2);

        scheduler.clear();

        assert_eq!(scheduler.pending_jobs(), 0);

        let stats = scheduler.stats();
        assert_eq!(stats.jobs_submitted, 2);
        assert_eq!(stats.jobs_cancelled, 2);
    }

    #[test]
    fn test_peek_next_job() {
        let scheduler = JobScheduler::new();

        assert!(scheduler.peek_next_job().is_none());

        let (job_id, _token) = scheduler.submit(
            JobPriority::Visible,
            JobType::LoadFile {
                path: PathBuf::from("test.pdf"),
            },
        );

        let peeked = scheduler.peek_next_job().unwrap();
        assert_eq!(peeked.id, job_id);

        // Peek shouldn't remove the job
        assert_eq!(scheduler.pending_jobs(), 1);
    }

    #[test]
    fn test_stats() {
        let scheduler = JobScheduler::new();

        scheduler.submit(
            JobPriority::Visible,
            JobType::LoadFile {
                path: PathBuf::from("test.pdf"),
            },
        );
        scheduler.submit(JobPriority::Ocr, JobType::RunOcr { page_index: 0 });

        let job1 = scheduler.next_job().unwrap();
        scheduler.complete_job(job1.id);

        let (job2_id, _token) = scheduler.submit(
            JobPriority::Margin,
            JobType::RenderTile {
                page_index: 0,
                tile_x: 0,
                tile_y: 0,
                zoom_level: 100,
                rotation: 0,
                is_preview: true,
            },
        );
        scheduler.cancel_job(job2_id);

        let stats = scheduler.stats();
        assert_eq!(stats.jobs_submitted, 3);
        assert_eq!(stats.jobs_completed, 1);
        assert_eq!(stats.jobs_cancelled, 1);
        assert_eq!(stats.queue_size, 1);
        assert_eq!(stats.pending_jobs(), 1);
    }

    #[test]
    fn test_default() {
        let scheduler = JobScheduler::default();
        assert_eq!(scheduler.pending_jobs(), 0);
    }

    #[test]
    fn test_pending_jobs_list() {
        let scheduler = JobScheduler::new();

        scheduler.submit(
            JobPriority::Visible,
            JobType::LoadFile {
                path: PathBuf::from("test.pdf"),
            },
        );
        scheduler.submit(JobPriority::Ocr, JobType::RunOcr { page_index: 0 });

        let jobs = scheduler.pending_jobs_list();
        assert_eq!(jobs.len(), 2);
    }

    #[test]
    fn test_cancellation_token_on_submit() {
        let scheduler = JobScheduler::new();

        let (job_id, token) = scheduler.submit(
            JobPriority::Visible,
            JobType::LoadFile {
                path: PathBuf::from("test.pdf"),
            },
        );

        // Token should not be cancelled initially
        assert!(!token.is_cancelled());

        // Cancel the job
        scheduler.cancel_job(job_id);

        // Token should now be cancelled
        assert!(token.is_cancelled());
    }

    #[test]
    fn test_cancellation_token_running_job() {
        let scheduler = JobScheduler::new();

        let (job_id, token) = scheduler.submit(
            JobPriority::Visible,
            JobType::LoadFile {
                path: PathBuf::from("test.pdf"),
            },
        );

        // Retrieve job (starts running)
        let job = scheduler.next_job().unwrap();
        assert_eq!(job.id, job_id);

        // Cancel should still work for running jobs
        let cancelled = scheduler.cancel_job(job_id);
        assert!(cancelled);
        assert!(token.is_cancelled());

        // Complete the job
        scheduler.complete_job(job_id);

        // Token should still be cancelled
        assert!(token.is_cancelled());
    }

    #[test]
    fn test_get_cancellation_token() {
        let scheduler = JobScheduler::new();

        let (job_id, token1) = scheduler.submit(
            JobPriority::Visible,
            JobType::LoadFile {
                path: PathBuf::from("test.pdf"),
            },
        );

        // Get token from scheduler
        let token2 = scheduler.get_cancellation_token(job_id).unwrap();

        // Cancel via original token
        token1.cancel();

        // Both tokens should observe cancellation
        assert!(token1.is_cancelled());
        assert!(token2.is_cancelled());
    }

    #[test]
    fn test_cancel_page_jobs_with_tokens() {
        let scheduler = JobScheduler::new();

        let (_id1, token1) = scheduler.submit(
            JobPriority::Visible,
            JobType::RenderTile {
                page_index: 0,
                tile_x: 0,
                tile_y: 0,
                zoom_level: 100,
                rotation: 0,
                is_preview: true,
            },
        );
        let (_id2, token2) = scheduler.submit(
            JobPriority::Visible,
            JobType::RenderTile {
                page_index: 1,
                tile_x: 0,
                tile_y: 0,
                zoom_level: 100,
                rotation: 0,
                is_preview: true,
            },
        );

        // Cancel page 0 jobs
        let cancelled = scheduler.cancel_page_jobs(0);
        assert_eq!(cancelled, 1);

        // Token for page 0 should be cancelled
        assert!(token1.is_cancelled());
        // Token for page 1 should not be cancelled
        assert!(!token2.is_cancelled());
    }

    #[test]
    fn test_clear_with_tokens() {
        let scheduler = JobScheduler::new();

        let (_id1, token1) = scheduler.submit(
            JobPriority::Visible,
            JobType::LoadFile {
                path: PathBuf::from("test.pdf"),
            },
        );
        let (_id2, token2) = scheduler.submit(JobPriority::Ocr, JobType::RunOcr { page_index: 0 });

        assert_eq!(scheduler.pending_jobs(), 2);

        scheduler.clear();

        assert_eq!(scheduler.pending_jobs(), 0);

        // Both tokens should be cancelled
        assert!(token1.is_cancelled());
        assert!(token2.is_cancelled());
    }

    #[test]
    fn test_cancel_offscreen_jobs() {
        use crate::viewport::Viewport;

        let scheduler = JobScheduler::new();

        // Viewport at origin, 800x600, zoom 100%
        // At 100% zoom with 256px tiles:
        // - Viewport covers tiles 0-3 horizontally (800/256 = 3.125 tiles)
        // - Viewport covers tiles 0-2 vertically (600/256 = 2.34 tiles)
        // - Margin of 1 tile extends to tile 4 horizontally, tile 3 vertically
        let viewport = Viewport::new(0, 0.0, 0.0, 800.0, 600.0, 100);

        // Submit visible tile (within viewport)
        let (_id1, token1) = scheduler.submit(
            JobPriority::Visible,
            JobType::RenderTile {
                page_index: 0,
                tile_x: 0,
                tile_y: 0,
                zoom_level: 100,
                rotation: 0,
                is_preview: true,
            },
        );

        // Submit margin tile (just outside viewport, at tile_x=4, within 1-tile margin)
        let (_id2, token2) = scheduler.submit(
            JobPriority::Margin,
            JobType::RenderTile {
                page_index: 0,
                tile_x: 4,
                tile_y: 0,
                zoom_level: 100,
                rotation: 0,
                is_preview: true,
            },
        );

        // Submit off-screen tile (far from viewport, at tile_x=50)
        let (_id3, token3) = scheduler.submit(
            JobPriority::Thumbnails,
            JobType::RenderTile {
                page_index: 0,
                tile_x: 50,
                tile_y: 0,
                zoom_level: 100,
                rotation: 0,
                is_preview: true,
            },
        );

        // Submit OCR job (should be cancelled)
        let (_id4, token4) = scheduler.submit(JobPriority::Ocr, JobType::RunOcr { page_index: 0 });

        // Submit file load job (should not be cancelled)
        let (_id5, token5) = scheduler.submit(
            JobPriority::Visible,
            JobType::LoadFile {
                path: PathBuf::from("test.pdf"),
            },
        );

        assert_eq!(scheduler.pending_jobs(), 5);

        // Cancel off-screen jobs
        let cancelled = scheduler.cancel_offscreen_jobs(&viewport, 256);

        // Should cancel off-screen tile and OCR job (2 jobs)
        assert_eq!(cancelled, 2);
        assert_eq!(scheduler.pending_jobs(), 3);

        // Visible and margin tiles should not be cancelled
        assert!(!token1.is_cancelled());
        assert!(!token2.is_cancelled());

        // Off-screen tile should be cancelled
        assert!(token3.is_cancelled());

        // OCR job should be cancelled
        assert!(token4.is_cancelled());

        // File load job should not be cancelled
        assert!(!token5.is_cancelled());
    }

    #[test]
    fn test_cancel_all_except_visible() {
        use crate::viewport::Viewport;

        let scheduler = JobScheduler::new();

        // Viewport at origin, 800x600, zoom 100%
        let viewport = Viewport::new(0, 0.0, 0.0, 800.0, 600.0, 100);

        // Submit visible tile (within viewport)
        let (_id1, token1) = scheduler.submit(
            JobPriority::Visible,
            JobType::RenderTile {
                page_index: 0,
                tile_x: 0,
                tile_y: 0,
                zoom_level: 100,
                rotation: 0,
                is_preview: true,
            },
        );

        // Submit margin tile (just outside viewport)
        let (_id2, token2) = scheduler.submit(
            JobPriority::Margin,
            JobType::RenderTile {
                page_index: 0,
                tile_x: 10,
                tile_y: 0,
                zoom_level: 100,
                rotation: 0,
                is_preview: true,
            },
        );

        // Submit off-screen tile
        let (_id3, token3) = scheduler.submit(
            JobPriority::Thumbnails,
            JobType::RenderTile {
                page_index: 0,
                tile_x: 50,
                tile_y: 0,
                zoom_level: 100,
                rotation: 0,
                is_preview: true,
            },
        );

        // Submit thumbnail job
        let (_id4, token4) = scheduler.submit(
            JobPriority::Thumbnails,
            JobType::GenerateThumbnail {
                page_index: 0,
                width: 100,
                height: 100,
            },
        );

        // Submit OCR job
        let (_id5, token5) = scheduler.submit(JobPriority::Ocr, JobType::RunOcr { page_index: 0 });

        // Submit file load job (should not be cancelled)
        let (_id6, token6) = scheduler.submit(
            JobPriority::Visible,
            JobType::LoadFile {
                path: PathBuf::from("test.pdf"),
            },
        );

        assert_eq!(scheduler.pending_jobs(), 6);

        // Aggressively cancel all except visible
        let cancelled = scheduler.cancel_all_except_visible(&viewport, 256);

        // Should cancel margin tile, off-screen tile, thumbnail, and OCR (4 jobs)
        assert_eq!(cancelled, 4);
        assert_eq!(scheduler.pending_jobs(), 2);

        // Only visible tile should not be cancelled
        assert!(!token1.is_cancelled());

        // Margin tile should be cancelled (aggressive mode)
        assert!(token2.is_cancelled());

        // Off-screen tile should be cancelled
        assert!(token3.is_cancelled());

        // Thumbnail should be cancelled
        assert!(token4.is_cancelled());

        // OCR should be cancelled
        assert!(token5.is_cancelled());

        // File load job should not be cancelled
        assert!(!token6.is_cancelled());
    }

    #[test]
    fn test_cancel_offscreen_jobs_with_page_change() {
        use crate::viewport::Viewport;

        let scheduler = JobScheduler::new();

        // Submit tiles for page 0
        let (_id1, token1) = scheduler.submit(
            JobPriority::Visible,
            JobType::RenderTile {
                page_index: 0,
                tile_x: 0,
                tile_y: 0,
                zoom_level: 100,
                rotation: 0,
                is_preview: true,
            },
        );

        // Submit tiles for page 2 (distant page)
        let (_id2, token2) = scheduler.submit(
            JobPriority::Adjacent,
            JobType::RenderTile {
                page_index: 2,
                tile_x: 0,
                tile_y: 0,
                zoom_level: 100,
                rotation: 0,
                is_preview: true,
            },
        );

        assert_eq!(scheduler.pending_jobs(), 2);

        // Change viewport to page 5
        let viewport = Viewport::new(5, 0.0, 0.0, 800.0, 600.0, 100);

        // Cancel off-screen jobs for new viewport
        let cancelled = scheduler.cancel_offscreen_jobs(&viewport, 256);

        // Both jobs should be cancelled (not visible or adjacent to page 5)
        assert_eq!(cancelled, 2);
        assert_eq!(scheduler.pending_jobs(), 0);

        assert!(token1.is_cancelled());
        assert!(token2.is_cancelled());
    }

    #[test]
    fn test_cancel_offscreen_jobs_with_zoom_change() {
        use crate::viewport::Viewport;

        let scheduler = JobScheduler::new();

        // Submit tile at 100% zoom
        let (_id1, token1) = scheduler.submit(
            JobPriority::Visible,
            JobType::RenderTile {
                page_index: 0,
                tile_x: 0,
                tile_y: 0,
                zoom_level: 100,
                rotation: 0,
                is_preview: true,
            },
        );

        // Submit tile at 200% zoom
        let (_id2, token2) = scheduler.submit(
            JobPriority::Visible,
            JobType::RenderTile {
                page_index: 0,
                tile_x: 0,
                tile_y: 0,
                zoom_level: 200,
                rotation: 0,
                is_preview: true,
            },
        );

        assert_eq!(scheduler.pending_jobs(), 2);

        // Change viewport to 200% zoom
        let viewport = Viewport::new(0, 0.0, 0.0, 800.0, 600.0, 200);

        // Cancel off-screen jobs for new viewport (different zoom)
        let cancelled = scheduler.cancel_offscreen_jobs(&viewport, 256);

        // Should cancel the 100% zoom tile (1 job)
        assert_eq!(cancelled, 1);
        assert_eq!(scheduler.pending_jobs(), 1);

        // 100% zoom tile should be cancelled
        assert!(token1.is_cancelled());

        // 200% zoom tile should not be cancelled
        assert!(!token2.is_cancelled());
    }

    #[test]
    fn test_cancel_offscreen_jobs_keeps_adjacent_thumbnails() {
        use crate::viewport::Viewport;

        let scheduler = JobScheduler::new();

        // Viewport on page 5
        let viewport = Viewport::new(5, 0.0, 0.0, 800.0, 600.0, 100);

        // Submit thumbnail for current page
        let (_id1, token1) = scheduler.submit(
            JobPriority::Margin,
            JobType::GenerateThumbnail {
                page_index: 5,
                width: 100,
                height: 100,
            },
        );

        // Submit thumbnail for adjacent page
        let (_id2, token2) = scheduler.submit(
            JobPriority::Adjacent,
            JobType::GenerateThumbnail {
                page_index: 6,
                width: 100,
                height: 100,
            },
        );

        // Submit thumbnail for distant page
        let (_id3, token3) = scheduler.submit(
            JobPriority::Thumbnails,
            JobType::GenerateThumbnail {
                page_index: 10,
                width: 100,
                height: 100,
            },
        );

        assert_eq!(scheduler.pending_jobs(), 3);

        // Cancel off-screen jobs
        let cancelled = scheduler.cancel_offscreen_jobs(&viewport, 256);

        // Should cancel only the distant page thumbnail (1 job)
        assert_eq!(cancelled, 1);
        assert_eq!(scheduler.pending_jobs(), 2);

        // Current page thumbnail should not be cancelled
        assert!(!token1.is_cancelled());

        // Adjacent page thumbnail should not be cancelled
        assert!(!token2.is_cancelled());

        // Distant page thumbnail should be cancelled
        assert!(token3.is_cancelled());
    }
}
