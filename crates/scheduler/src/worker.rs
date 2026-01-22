//! Render worker pool for parallel job execution.
//!
//! This module provides a thread pool of render workers that execute jobs
//! from the job scheduler. Workers run independently on separate threads,
//! pulling jobs from the scheduler, executing them, and checking for
//! cancellation.

use crate::{CancellationToken, Job, JobScheduler};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::thread::{self, JoinHandle};
use std::time::Duration;

/// Callback function for job execution.
///
/// Workers invoke this callback for each job they pull from the scheduler.
/// The callback receives the job to execute and its cancellation token.
/// It should check `token.is_cancelled()` periodically during execution
/// and return early if the job has been cancelled.
///
/// # Arguments
///
/// * `job` - The job to execute
/// * `token` - Cancellation token for cooperative cancellation
pub type JobExecutor = Arc<dyn Fn(&Job, &CancellationToken) + Send + Sync>;

/// Configuration for the render worker pool.
#[derive(Debug, Clone)]
pub struct WorkerPoolConfig {
    /// Number of worker threads to spawn.
    /// Default: number of logical CPU cores.
    pub num_workers: usize,

    /// Maximum time a worker will wait for a job before checking shutdown.
    /// Default: 100ms.
    pub poll_interval: Duration,
}

impl Default for WorkerPoolConfig {
    fn default() -> Self {
        Self {
            num_workers: num_cpus(),
            poll_interval: Duration::from_millis(100),
        }
    }
}

impl WorkerPoolConfig {
    /// Create a new worker pool configuration.
    pub fn new(num_workers: usize) -> Self {
        Self {
            num_workers,
            poll_interval: Duration::from_millis(100),
        }
    }

    /// Set the poll interval for workers.
    pub fn with_poll_interval(mut self, interval: Duration) -> Self {
        self.poll_interval = interval;
        self
    }
}

/// Render worker pool for parallel job execution.
///
/// The worker pool spawns multiple worker threads that execute jobs from
/// the job scheduler in parallel. Each worker pulls jobs from the scheduler,
/// executes them via the job executor callback, and marks them as complete.
///
/// Workers support cooperative cancellation by checking cancellation tokens
/// during execution.
///
/// # Example
///
/// ```
/// use pdf_editor_scheduler::{Job, CancellationToken, JobScheduler, JobPriority, JobType, WorkerPool, WorkerPoolConfig};
/// use std::sync::Arc;
///
/// let scheduler = Arc::new(JobScheduler::new());
/// let scheduler_clone = scheduler.clone();
///
/// // Create a job executor callback
/// let executor = Arc::new(move |job: &Job, token: &CancellationToken| {
///     match &job.job_type {
///         JobType::RenderTile { page_index, tile_x, tile_y, .. } => {
///             println!("Rendering tile ({}, {}) on page {}", tile_x, tile_y, page_index);
///             // Check for cancellation during rendering
///             if token.is_cancelled() {
///                 println!("Job {} was cancelled", job.id);
///                 return;
///             }
///             // ... perform rendering ...
///         }
///         _ => {}
///     }
/// });
///
/// // Start worker pool with default config
/// let pool = WorkerPool::new(scheduler_clone, executor, WorkerPoolConfig::default());
///
/// // Submit some jobs
/// scheduler.submit(JobPriority::Visible, JobType::RenderTile {
///     page_index: 0,
///     tile_x: 0,
///     tile_y: 0,
///     zoom_level: 100,
///     rotation: 0,
///     is_preview: false,
/// });
///
/// // Workers process jobs in the background...
///
/// // Shutdown when done
/// pool.shutdown();
/// ```
pub struct WorkerPool {
    workers: Vec<Worker>,
    shutdown: Arc<AtomicBool>,
}

impl WorkerPool {
    /// Create and start a new worker pool.
    ///
    /// # Arguments
    ///
    /// * `scheduler` - Job scheduler to pull jobs from
    /// * `executor` - Job executor callback for executing jobs
    /// * `config` - Worker pool configuration
    pub fn new(
        scheduler: Arc<JobScheduler>,
        executor: JobExecutor,
        config: WorkerPoolConfig,
    ) -> Self {
        let shutdown = Arc::new(AtomicBool::new(false));
        let mut workers = Vec::with_capacity(config.num_workers);

        for id in 0..config.num_workers {
            let worker = Worker::new(
                id,
                scheduler.clone(),
                executor.clone(),
                shutdown.clone(),
                config.poll_interval,
            );
            workers.push(worker);
        }

        Self { workers, shutdown }
    }

    /// Get the number of worker threads.
    pub fn num_workers(&self) -> usize {
        self.workers.len()
    }

    /// Check if the worker pool is shutting down.
    pub fn is_shutting_down(&self) -> bool {
        self.shutdown.load(Ordering::Acquire)
    }

    /// Shutdown the worker pool gracefully.
    ///
    /// This signals all workers to stop and waits for them to finish
    /// their current jobs and exit. This method blocks until all workers
    /// have terminated.
    pub fn shutdown(self) {
        // Signal shutdown
        self.shutdown.store(true, Ordering::Release);

        // Wait for all workers to finish
        for worker in self.workers {
            worker.join();
        }
    }

    /// Shutdown the worker pool without waiting.
    ///
    /// This signals all workers to stop but does not wait for them to finish.
    /// Workers will complete their current jobs and exit in the background.
    pub fn shutdown_nowait(self) {
        self.shutdown.store(true, Ordering::Release);
    }
}

/// A single worker thread in the worker pool.
struct Worker {
    thread: Option<JoinHandle<()>>,
}

impl Worker {
    /// Create and start a new worker thread.
    fn new(
        id: usize,
        scheduler: Arc<JobScheduler>,
        executor: JobExecutor,
        shutdown: Arc<AtomicBool>,
        poll_interval: Duration,
    ) -> Self {
        let thread = thread::Builder::new()
            .name(format!("pdf-render-worker-{}", id))
            .spawn(move || {
                Self::run(scheduler, executor, shutdown, poll_interval);
            })
            .expect("Failed to spawn worker thread");

        Self {
            thread: Some(thread),
        }
    }

    /// Main worker loop.
    ///
    /// Workers continuously pull jobs from the scheduler, execute them,
    /// and mark them as complete. They check for shutdown signals between
    /// jobs and sleep briefly if no jobs are available.
    fn run(
        scheduler: Arc<JobScheduler>,
        executor: JobExecutor,
        shutdown: Arc<AtomicBool>,
        poll_interval: Duration,
    ) {
        loop {
            // Check for shutdown signal
            if shutdown.load(Ordering::Acquire) {
                break;
            }

            // Try to get the next job
            if let Some(job) = scheduler.next_job() {
                let job_id = job.id;

                // Get the cancellation token for this job
                let token = scheduler.get_cancellation_token(job_id).unwrap_or_default();

                // Check if the job was already cancelled before execution
                if !token.is_cancelled() {
                    // Execute the job
                    executor(&job, &token);
                }

                // Mark job as complete
                scheduler.complete_job(job_id);
            } else {
                // No jobs available, sleep briefly
                thread::sleep(poll_interval);
            }
        }
    }

    /// Wait for the worker thread to finish.
    fn join(mut self) {
        if let Some(thread) = self.thread.take() {
            thread.join().expect("Worker thread panicked");
        }
    }
}

/// Get the number of logical CPU cores.
///
/// This is used as the default number of worker threads.
fn num_cpus() -> usize {
    thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{JobPriority, JobType};
    use std::sync::atomic::AtomicUsize;
    use std::sync::Mutex;

    #[test]
    fn test_worker_pool_config_default() {
        let config = WorkerPoolConfig::default();
        assert!(config.num_workers > 0);
        assert_eq!(config.poll_interval, Duration::from_millis(100));
    }

    #[test]
    fn test_worker_pool_config_new() {
        let config = WorkerPoolConfig::new(8);
        assert_eq!(config.num_workers, 8);
    }

    #[test]
    fn test_worker_pool_config_builder() {
        let config = WorkerPoolConfig::new(4).with_poll_interval(Duration::from_millis(50));
        assert_eq!(config.num_workers, 4);
        assert_eq!(config.poll_interval, Duration::from_millis(50));
    }

    #[test]
    fn test_worker_pool_creation() {
        let scheduler = Arc::new(JobScheduler::new());
        let executor = Arc::new(|_job: &Job, _token: &CancellationToken| {});
        let config = WorkerPoolConfig::new(2);

        let pool = WorkerPool::new(scheduler, executor, config);
        assert_eq!(pool.num_workers(), 2);
        assert!(!pool.is_shutting_down());

        pool.shutdown();
    }

    #[test]
    fn test_worker_pool_executes_jobs() {
        let scheduler = Arc::new(JobScheduler::new());
        let executed = Arc::new(AtomicUsize::new(0));
        let executed_clone = executed.clone();

        let executor = Arc::new(move |_job: &Job, _token: &CancellationToken| {
            executed_clone.fetch_add(1, Ordering::SeqCst);
            thread::sleep(Duration::from_millis(10));
        });

        let config = WorkerPoolConfig::new(2);
        let pool = WorkerPool::new(scheduler.clone(), executor, config);

        // Submit 5 jobs
        for i in 0..5 {
            scheduler.submit(
                JobPriority::Visible,
                JobType::RenderTile {
                    page_index: i,
                    tile_x: 0,
                    tile_y: 0,
                    zoom_level: 100,
                    rotation: 0,
                    is_preview: false,
                },
            );
        }

        // Wait for jobs to complete
        thread::sleep(Duration::from_millis(200));

        // All jobs should be executed
        assert_eq!(executed.load(Ordering::SeqCst), 5);

        pool.shutdown();
    }

    #[test]
    fn test_worker_pool_respects_cancellation() {
        let scheduler = Arc::new(JobScheduler::new());
        let started = Arc::new(AtomicUsize::new(0));
        let completed = Arc::new(AtomicUsize::new(0));
        let started_clone = started.clone();
        let completed_clone = completed.clone();

        let executor = Arc::new(move |_job: &Job, token: &CancellationToken| {
            started_clone.fetch_add(1, Ordering::SeqCst);

            // Simulate work with cancellation checks
            for _ in 0..10 {
                if token.is_cancelled() {
                    return; // Exit early if cancelled
                }
                thread::sleep(Duration::from_millis(10));
            }

            completed_clone.fetch_add(1, Ordering::SeqCst);
        });

        let config = WorkerPoolConfig::new(1);
        let pool = WorkerPool::new(scheduler.clone(), executor, config);

        // Submit 3 jobs
        let mut job_ids = Vec::new();
        for i in 0..3 {
            let (job_id, _token) = scheduler.submit(
                JobPriority::Visible,
                JobType::RenderTile {
                    page_index: i,
                    tile_x: 0,
                    tile_y: 0,
                    zoom_level: 100,
                    rotation: 0,
                    is_preview: false,
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

        pool.shutdown();
    }

    #[test]
    fn test_worker_pool_priority_ordering() {
        let scheduler = Arc::new(JobScheduler::new());
        let execution_order = Arc::new(Mutex::new(Vec::new()));
        let execution_order_clone = execution_order.clone();

        let executor = Arc::new(move |job: &Job, _token: &CancellationToken| {
            if let JobType::RenderTile { page_index, .. } = job.job_type {
                execution_order_clone.lock().unwrap().push(page_index);
            }
            thread::sleep(Duration::from_millis(10));
        });

        let config = WorkerPoolConfig::new(1); // Single worker for deterministic ordering
        let pool = WorkerPool::new(scheduler.clone(), executor, config);

        // Submit jobs with different priorities
        scheduler.submit(
            JobPriority::Ocr,
            JobType::RenderTile {
                page_index: 3,
                tile_x: 0,
                tile_y: 0,
                zoom_level: 100,
                rotation: 0,
                is_preview: false,
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
                is_preview: false,
            },
        );
        scheduler.submit(
            JobPriority::Adjacent,
            JobType::RenderTile {
                page_index: 2,
                tile_x: 0,
                tile_y: 0,
                zoom_level: 100,
                rotation: 0,
                is_preview: false,
            },
        );

        // Wait for all jobs to complete
        thread::sleep(Duration::from_millis(150));

        let order = execution_order.lock().unwrap();
        assert_eq!(*order, vec![1, 2, 3]); // Visible > Adjacent > Ocr

        pool.shutdown();
    }

    #[test]
    fn test_worker_pool_shutdown() {
        let scheduler = Arc::new(JobScheduler::new());
        let executor = Arc::new(|_job: &Job, _token: &CancellationToken| {
            thread::sleep(Duration::from_millis(10));
        });

        let config = WorkerPoolConfig::new(2);
        let pool = WorkerPool::new(scheduler, executor, config);

        assert!(!pool.is_shutting_down());

        pool.shutdown();
        // Shutdown is successful if this completes without hanging
    }

    #[test]
    fn test_num_cpus() {
        let cpus = num_cpus();
        assert!(cpus > 0);
        assert!(cpus <= 1024); // Sanity check
    }
}
