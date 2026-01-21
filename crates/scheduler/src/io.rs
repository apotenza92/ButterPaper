//! IO thread for file operations.
//!
//! This module provides a dedicated IO thread for handling file operations
//! separately from the render worker pool. File operations (loading PDFs,
//! reading disk cache, etc.) are IO-bound rather than CPU-bound, so they
//! benefit from a separate thread to avoid blocking render workers.

use crate::{CancellationToken, Job, JobScheduler, JobType};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::thread::{self, JoinHandle};
use std::time::Duration;

/// Callback function for IO operations.
///
/// The IO thread invokes this callback for each IO job it pulls from the scheduler.
/// The callback receives the job to execute and its cancellation token.
/// It should check `token.is_cancelled()` periodically during execution
/// and return early if the job has been cancelled.
///
/// # Arguments
///
/// * `job` - The job to execute
/// * `token` - Cancellation token for cooperative cancellation
pub type IoExecutor = Arc<dyn Fn(&Job, &CancellationToken) + Send + Sync>;

/// Configuration for the IO thread.
#[derive(Debug, Clone)]
pub struct IoThreadConfig {
    /// Maximum time the IO thread will wait for a job before checking shutdown.
    /// Default: 100ms.
    pub poll_interval: Duration,
}

impl Default for IoThreadConfig {
    fn default() -> Self {
        Self {
            poll_interval: Duration::from_millis(100),
        }
    }
}

impl IoThreadConfig {
    /// Create a new IO thread configuration.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the poll interval for the IO thread.
    pub fn with_poll_interval(mut self, interval: Duration) -> Self {
        self.poll_interval = interval;
        self
    }
}

/// Dedicated IO thread for file operations.
///
/// The IO thread handles file operations (loading PDFs, reading disk cache, etc.)
/// separately from the render worker pool. This prevents IO-bound operations from
/// blocking CPU-bound rendering work.
///
/// The IO thread only processes jobs of type `LoadFile`. All other job types are
/// ignored and left in the queue for render workers to handle.
///
/// # Example
///
/// ```
/// use pdf_editor_scheduler::{Job, CancellationToken, JobScheduler, JobPriority, JobType, IoThread, IoThreadConfig};
/// use std::sync::Arc;
/// use std::path::PathBuf;
///
/// let scheduler = Arc::new(JobScheduler::new());
/// let scheduler_clone = scheduler.clone();
///
/// // Create an IO executor callback
/// let executor = Arc::new(move |job: &Job, token: &CancellationToken| {
///     match &job.job_type {
///         JobType::LoadFile { path } => {
///             println!("Loading file: {:?}", path);
///             // Check for cancellation during file loading
///             if token.is_cancelled() {
///                 println!("Job {} was cancelled", job.id);
///                 return;
///             }
///             // ... perform file loading ...
///         }
///         _ => {}
///     }
/// });
///
/// // Start IO thread with default config
/// let io_thread = IoThread::new(scheduler_clone, executor, IoThreadConfig::default());
///
/// // Submit a file loading job
/// scheduler.submit(JobPriority::Visible, JobType::LoadFile {
///     path: PathBuf::from("/path/to/document.pdf"),
/// });
///
/// // IO thread processes the job in the background...
///
/// // Shutdown when done
/// io_thread.shutdown();
/// ```
pub struct IoThread {
    thread: Option<JoinHandle<()>>,
    shutdown: Arc<AtomicBool>,
}

impl IoThread {
    /// Create and start a new IO thread.
    ///
    /// # Arguments
    ///
    /// * `scheduler` - Job scheduler to pull jobs from
    /// * `executor` - IO executor callback for executing IO jobs
    /// * `config` - IO thread configuration
    pub fn new(
        scheduler: Arc<JobScheduler>,
        executor: IoExecutor,
        config: IoThreadConfig,
    ) -> Self {
        let shutdown = Arc::new(AtomicBool::new(false));
        let shutdown_clone = shutdown.clone();

        let thread = thread::Builder::new()
            .name("pdf-io-thread".to_string())
            .spawn(move || {
                Self::run(scheduler, executor, shutdown_clone, config.poll_interval);
            })
            .expect("Failed to spawn IO thread");

        Self {
            thread: Some(thread),
            shutdown,
        }
    }

    /// Check if the IO thread is shutting down.
    pub fn is_shutting_down(&self) -> bool {
        self.shutdown.load(Ordering::Acquire)
    }

    /// Shutdown the IO thread gracefully.
    ///
    /// This signals the IO thread to stop and waits for it to finish
    /// its current job and exit. This method blocks until the thread
    /// has terminated.
    pub fn shutdown(mut self) {
        // Signal shutdown
        self.shutdown.store(true, Ordering::Release);

        // Wait for thread to finish
        if let Some(thread) = self.thread.take() {
            thread.join().expect("IO thread panicked");
        }
    }

    /// Main IO thread loop.
    ///
    /// The IO thread continuously pulls jobs from the scheduler, checking
    /// if they are IO jobs (LoadFile). If so, it executes them via the
    /// executor callback and marks them as complete. Non-IO jobs are left
    /// in the queue for render workers.
    fn run(
        scheduler: Arc<JobScheduler>,
        executor: IoExecutor,
        shutdown: Arc<AtomicBool>,
        poll_interval: Duration,
    ) {
        loop {
            // Check for shutdown signal
            if shutdown.load(Ordering::Acquire) {
                break;
            }

            // Peek at the next job without removing it
            if let Some(job) = scheduler.peek_next_job() {
                // Check if this is an IO job
                if matches!(job.job_type, JobType::LoadFile { .. }) {
                    // Remove the job from the queue
                    let job = scheduler
                        .next_job()
                        .expect("Job disappeared between peek and next");
                    let job_id = job.id;

                    // Get the cancellation token for this job
                    let token = scheduler
                        .get_cancellation_token(job_id)
                        .unwrap_or_default();

                    // Check if the job was already cancelled before execution
                    if !token.is_cancelled() {
                        // Execute the IO job
                        executor(&job, &token);
                    }

                    // Mark job as complete
                    scheduler.complete_job(job_id);
                } else {
                    // Not an IO job, leave it for render workers
                    thread::sleep(poll_interval);
                }
            } else {
                // No jobs available, sleep briefly
                thread::sleep(poll_interval);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{JobPriority, JobType};
    use std::path::PathBuf;
    use std::sync::atomic::AtomicUsize;
    use std::sync::Mutex;

    #[test]
    fn test_io_thread_config_default() {
        let config = IoThreadConfig::default();
        assert_eq!(config.poll_interval, Duration::from_millis(100));
    }

    #[test]
    fn test_io_thread_config_new() {
        let config = IoThreadConfig::new();
        assert_eq!(config.poll_interval, Duration::from_millis(100));
    }

    #[test]
    fn test_io_thread_config_builder() {
        let config = IoThreadConfig::new().with_poll_interval(Duration::from_millis(50));
        assert_eq!(config.poll_interval, Duration::from_millis(50));
    }

    #[test]
    fn test_io_thread_creation() {
        let scheduler = Arc::new(JobScheduler::new());
        let executor = Arc::new(|_job: &Job, _token: &CancellationToken| {});
        let config = IoThreadConfig::default();

        let io_thread = IoThread::new(scheduler, executor, config);
        assert!(!io_thread.is_shutting_down());

        io_thread.shutdown();
    }

    #[test]
    fn test_io_thread_executes_load_file_jobs() {
        let scheduler = Arc::new(JobScheduler::new());
        let executed = Arc::new(AtomicUsize::new(0));
        let executed_clone = executed.clone();

        let executor = Arc::new(move |job: &Job, _token: &CancellationToken| {
            if matches!(job.job_type, JobType::LoadFile { .. }) {
                executed_clone.fetch_add(1, Ordering::SeqCst);
                thread::sleep(Duration::from_millis(10));
            }
        });

        let config = IoThreadConfig::default();
        let io_thread = IoThread::new(scheduler.clone(), executor, config);

        // Submit 3 LoadFile jobs
        for i in 0..3 {
            scheduler.submit(
                JobPriority::Visible,
                JobType::LoadFile {
                    path: PathBuf::from(format!("/path/to/file{}.pdf", i)),
                },
            );
        }

        // Wait for jobs to complete
        thread::sleep(Duration::from_millis(150));

        // All LoadFile jobs should be executed
        assert_eq!(executed.load(Ordering::SeqCst), 3);

        io_thread.shutdown();
    }

    #[test]
    fn test_io_thread_ignores_non_io_jobs() {
        let scheduler = Arc::new(JobScheduler::new());
        let io_count = Arc::new(AtomicUsize::new(0));
        let io_count_clone = io_count.clone();

        let executor = Arc::new(move |job: &Job, _token: &CancellationToken| {
            if matches!(job.job_type, JobType::LoadFile { .. }) {
                io_count_clone.fetch_add(1, Ordering::SeqCst);
            }
        });

        let config = IoThreadConfig::default();
        let io_thread = IoThread::new(scheduler.clone(), executor, config);

        // Submit mixed jobs: LoadFile at high priority, RenderTile at lower priority, another LoadFile at high priority
        scheduler.submit(
            JobPriority::Visible,
            JobType::LoadFile {
                path: PathBuf::from("/path/to/file.pdf"),
            },
        );
        scheduler.submit(
            JobPriority::Margin,  // Lower priority so it doesn't block second LoadFile
            JobType::RenderTile {
                page_index: 0,
                tile_x: 0,
                tile_y: 0,
                zoom_level: 100,
                rotation: 0,
                is_preview: false,
            },
        );
        scheduler.submit(
            JobPriority::Visible,
            JobType::LoadFile {
                path: PathBuf::from("/path/to/file2.pdf"),
            },
        );

        // Wait for IO jobs to complete
        thread::sleep(Duration::from_millis(150));

        // Only LoadFile jobs should be executed by IO thread
        assert_eq!(io_count.load(Ordering::SeqCst), 2);

        // RenderTile job should still be in the queue
        let remaining_jobs = scheduler.pending_jobs_list();
        assert_eq!(remaining_jobs.len(), 1);
        assert!(matches!(
            remaining_jobs[0].job_type,
            JobType::RenderTile { .. }
        ));

        io_thread.shutdown();
    }

    #[test]
    fn test_io_thread_respects_cancellation() {
        let scheduler = Arc::new(JobScheduler::new());
        let started = Arc::new(AtomicUsize::new(0));
        let completed = Arc::new(AtomicUsize::new(0));
        let started_clone = started.clone();
        let completed_clone = completed.clone();

        let executor = Arc::new(move |job: &Job, token: &CancellationToken| {
            if matches!(job.job_type, JobType::LoadFile { .. }) {
                started_clone.fetch_add(1, Ordering::SeqCst);

                // Simulate work with cancellation checks
                for _ in 0..10 {
                    if token.is_cancelled() {
                        return; // Exit early if cancelled
                    }
                    thread::sleep(Duration::from_millis(10));
                }

                completed_clone.fetch_add(1, Ordering::SeqCst);
            }
        });

        let config = IoThreadConfig::default();
        let io_thread = IoThread::new(scheduler.clone(), executor, config);

        // Submit 3 jobs
        let mut job_ids = Vec::new();
        for i in 0..3 {
            let (job_id, _token) = scheduler.submit(
                JobPriority::Visible,
                JobType::LoadFile {
                    path: PathBuf::from(format!("/path/to/file{}.pdf", i)),
                },
            );
            job_ids.push(job_id);
        }

        // Wait for first job to start
        thread::sleep(Duration::from_millis(50));

        // Cancel remaining jobs
        for job_id in job_ids.iter().skip(1) {
            scheduler.cancel_job(*job_id);
        }

        // Wait for all jobs to finish
        thread::sleep(Duration::from_millis(300));

        // At least one job should have started
        assert!(started.load(Ordering::SeqCst) >= 1);
        // At most one job should have completed (others were cancelled)
        assert!(completed.load(Ordering::SeqCst) <= 1);

        io_thread.shutdown();
    }

    #[test]
    fn test_io_thread_priority_ordering() {
        let scheduler = Arc::new(JobScheduler::new());
        let execution_order = Arc::new(Mutex::new(Vec::new()));
        let execution_order_clone = execution_order.clone();

        let executor = Arc::new(move |job: &Job, _token: &CancellationToken| {
            if let JobType::LoadFile { path } = &job.job_type {
                let filename = path.file_name().unwrap().to_str().unwrap();
                execution_order_clone.lock().unwrap().push(filename.to_string());
            }
            thread::sleep(Duration::from_millis(10));
        });

        let config = IoThreadConfig::default();
        let io_thread = IoThread::new(scheduler.clone(), executor, config);

        // Submit jobs with different priorities
        scheduler.submit(
            JobPriority::Adjacent,
            JobType::LoadFile {
                path: PathBuf::from("file2.pdf"),
            },
        );
        scheduler.submit(
            JobPriority::Visible,
            JobType::LoadFile {
                path: PathBuf::from("file1.pdf"),
            },
        );
        scheduler.submit(
            JobPriority::Ocr,
            JobType::LoadFile {
                path: PathBuf::from("file3.pdf"),
            },
        );

        // Wait for all jobs to complete
        thread::sleep(Duration::from_millis(150));

        let order = execution_order.lock().unwrap();
        assert_eq!(*order, vec!["file1.pdf", "file2.pdf", "file3.pdf"]); // Visible > Adjacent > Ocr

        io_thread.shutdown();
    }

    #[test]
    fn test_io_thread_shutdown() {
        let scheduler = Arc::new(JobScheduler::new());
        let executor = Arc::new(|_job: &Job, _token: &CancellationToken| {
            thread::sleep(Duration::from_millis(10));
        });

        let config = IoThreadConfig::default();
        let io_thread = IoThread::new(scheduler, executor, config);

        assert!(!io_thread.is_shutting_down());

        io_thread.shutdown();
        // Shutdown is successful if this completes without hanging
    }
}
