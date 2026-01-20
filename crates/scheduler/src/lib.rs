//! PDF Editor Scheduler Library
//!
//! Job scheduler with priority queue and cancellable workers.
//!
//! This crate provides a priority-based job scheduling system for the PDF editor.
//! Jobs are organized by priority (visible tiles, margin tiles, adjacent pages,
//! thumbnails, OCR) and executed in priority order with FIFO ordering within
//! each priority level.
//!
//! # Example
//!
//! ```
//! use pdf_editor_scheduler::{JobScheduler, JobPriority, JobType};
//!
//! let scheduler = JobScheduler::new();
//!
//! // Submit a high-priority rendering job
//! let (job_id, token) = scheduler.submit(
//!     JobPriority::Visible,
//!     JobType::RenderTile {
//!         page_index: 0,
//!         tile_x: 0,
//!         tile_y: 0,
//!         zoom_level: 100,
//!         rotation: 0,
//!         is_preview: false,
//!     }
//! );
//!
//! // Get the next job to execute
//! if let Some(job) = scheduler.next_job() {
//!     println!("Executing job {}", job.id);
//!     // Worker can check token.is_cancelled() during execution
//!     // ... execute the job ...
//!     scheduler.complete_job(job.id);
//! }
//!
//! // Cancel jobs for a specific page when navigating away
//! scheduler.cancel_page_jobs(0);
//! ```

mod cancel;
mod priority;
mod scheduler;

// Re-export public API
pub use cancel::{CancellationRegistry, CancellationToken};
pub use priority::{Job, JobId, JobPriority, JobType};
pub use scheduler::{JobScheduler, SchedulerStats};
