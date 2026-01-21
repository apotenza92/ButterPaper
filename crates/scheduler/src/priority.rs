//! Priority-based job scheduling system
//!
//! Provides a priority queue for scheduling jobs with different priorities.
//! Jobs are executed in priority order, with higher priority jobs running first.

use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// Job priority levels
///
/// Higher numeric values have higher priority and are executed first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum JobPriority {
    /// OCR processing (lowest priority, runs when idle)
    Ocr = 0,

    /// Thumbnail generation (low priority)
    Thumbnails = 1,

    /// Adjacent pages (prefetch, medium-low priority)
    Adjacent = 2,

    /// Margin tiles (prefetch, medium priority)
    Margin = 3,

    /// Visible tiles (highest priority, must render immediately)
    Visible = 4,
}

/// Unique job identifier
pub type JobId = u64;

/// Job type enumeration
///
/// Defines the different types of jobs that can be scheduled.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JobType {
    /// Render a tile (includes tile coordinates, zoom, rotation, profile)
    RenderTile {
        page_index: u16,
        tile_x: u32,
        tile_y: u32,
        zoom_level: u32,
        rotation: u16,
        is_preview: bool,
    },

    /// Load a file from disk (includes file path)
    LoadFile { path: PathBuf },

    /// Generate page thumbnail (includes page index, size)
    GenerateThumbnail {
        page_index: u16,
        width: u32,
        height: u32,
    },

    /// Run OCR on a page (includes page index)
    RunOcr { page_index: u16 },

    /// Extract text from a page (includes page index)
    ExtractText { page_index: u16 },
}

/// A scheduled job with priority
///
/// Jobs are ordered by priority (higher priority first), then by insertion order
/// (earlier jobs first) to ensure FIFO ordering within the same priority level.
#[derive(Debug, Clone)]
pub struct Job {
    /// Unique job identifier
    pub id: JobId,

    /// Job priority level
    pub priority: JobPriority,

    /// Job type and parameters
    pub job_type: JobType,

    /// Insertion order (used for FIFO within same priority)
    insertion_order: u64,
}

impl Job {
    /// Create a new job
    pub fn new(id: JobId, priority: JobPriority, job_type: JobType, insertion_order: u64) -> Self {
        Self {
            id,
            priority,
            job_type,
            insertion_order,
        }
    }
}

impl PartialEq for Job {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for Job {}

impl PartialOrd for Job {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Job {
    fn cmp(&self, other: &Self) -> Ordering {
        // First compare by priority (higher priority first)
        match self.priority.cmp(&other.priority) {
            Ordering::Equal => {
                // Within same priority, compare by insertion order (earlier first)
                // Note: We reverse the ordering here because BinaryHeap is a max heap
                other.insertion_order.cmp(&self.insertion_order)
            }
            other => other,
        }
    }
}

/// Priority queue for jobs
///
/// Thread-safe job queue that orders jobs by priority and insertion order.
/// Higher priority jobs are dequeued first, and within the same priority level,
/// jobs are dequeued in FIFO order.
pub struct PriorityQueue {
    state: Arc<Mutex<QueueState>>,
}

struct QueueState {
    /// Binary heap for priority-ordered jobs (max heap)
    heap: BinaryHeap<Job>,

    /// Next job ID (for automatic ID assignment)
    next_job_id: JobId,

    /// Insertion counter (for FIFO ordering within same priority)
    insertion_counter: u64,
}

impl PriorityQueue {
    /// Create a new empty priority queue
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(QueueState {
                heap: BinaryHeap::new(),
                next_job_id: 1,
                insertion_counter: 0,
            })),
        }
    }

    /// Push a job onto the queue
    ///
    /// The job will be assigned a unique ID and inserted according to its priority.
    /// Returns the assigned job ID.
    pub fn push(&self, priority: JobPriority, job_type: JobType) -> JobId {
        let mut state = self.state.lock().unwrap();
        let job_id = state.next_job_id;
        state.next_job_id += 1;

        let insertion_order = state.insertion_counter;
        state.insertion_counter += 1;

        let job = Job::new(job_id, priority, job_type, insertion_order);
        state.heap.push(job);

        job_id
    }

    /// Pop the highest priority job from the queue
    ///
    /// Returns `None` if the queue is empty.
    pub fn pop(&self) -> Option<Job> {
        let mut state = self.state.lock().unwrap();
        state.heap.pop()
    }

    /// Peek at the highest priority job without removing it
    ///
    /// Returns `None` if the queue is empty.
    pub fn peek(&self) -> Option<Job> {
        let state = self.state.lock().unwrap();
        state.heap.peek().cloned()
    }

    /// Get the number of jobs in the queue
    pub fn len(&self) -> usize {
        let state = self.state.lock().unwrap();
        state.heap.len()
    }

    /// Check if the queue is empty
    pub fn is_empty(&self) -> bool {
        let state = self.state.lock().unwrap();
        state.heap.is_empty()
    }

    /// Clear all jobs from the queue
    pub fn clear(&self) {
        let mut state = self.state.lock().unwrap();
        state.heap.clear();
    }

    /// Remove all jobs matching a predicate
    ///
    /// Returns the number of jobs removed.
    pub fn remove_if<F>(&self, predicate: F) -> usize
    where
        F: Fn(&Job) -> bool,
    {
        let mut state = self.state.lock().unwrap();
        let original_len = state.heap.len();

        // Drain matching jobs into a temporary vector
        let mut remaining = Vec::new();
        while let Some(job) = state.heap.pop() {
            if !predicate(&job) {
                remaining.push(job);
            }
        }

        // Rebuild the heap with remaining jobs
        state.heap = remaining.into_iter().collect();

        original_len - state.heap.len()
    }

    /// Get all jobs currently in the queue (for debugging/inspection)
    ///
    /// Jobs are returned in arbitrary order (not priority order).
    pub fn jobs(&self) -> Vec<Job> {
        let state = self.state.lock().unwrap();
        state.heap.iter().cloned().collect()
    }
}

impl Default for PriorityQueue {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_job_priority_ordering() {
        assert!(JobPriority::Visible > JobPriority::Margin);
        assert!(JobPriority::Margin > JobPriority::Adjacent);
        assert!(JobPriority::Adjacent > JobPriority::Thumbnails);
        assert!(JobPriority::Thumbnails > JobPriority::Ocr);
    }

    #[test]
    fn test_priority_queue_basic() {
        let queue = PriorityQueue::new();

        assert!(queue.is_empty());
        assert_eq!(queue.len(), 0);

        let id1 = queue.push(
            JobPriority::Visible,
            JobType::LoadFile {
                path: PathBuf::from("test.pdf"),
            },
        );
        assert!(!queue.is_empty());
        assert_eq!(queue.len(), 1);

        let job = queue.pop().unwrap();
        assert_eq!(job.id, id1);
        assert_eq!(job.priority, JobPriority::Visible);

        assert!(queue.is_empty());
    }

    #[test]
    fn test_priority_queue_ordering() {
        let queue = PriorityQueue::new();

        // Insert jobs in reverse priority order
        queue.push(JobPriority::Ocr, JobType::RunOcr { page_index: 0 });
        queue.push(
            JobPriority::Thumbnails,
            JobType::GenerateThumbnail {
                page_index: 0,
                width: 100,
                height: 100,
            },
        );
        queue.push(
            JobPriority::Visible,
            JobType::LoadFile {
                path: PathBuf::from("test.pdf"),
            },
        );
        queue.push(
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

        // Pop jobs and verify they come out in priority order
        assert_eq!(queue.pop().unwrap().priority, JobPriority::Visible);
        assert_eq!(queue.pop().unwrap().priority, JobPriority::Margin);
        assert_eq!(queue.pop().unwrap().priority, JobPriority::Thumbnails);
        assert_eq!(queue.pop().unwrap().priority, JobPriority::Ocr);
        assert!(queue.pop().is_none());
    }

    #[test]
    fn test_fifo_within_same_priority() {
        let queue = PriorityQueue::new();

        // Insert multiple jobs with same priority
        let id1 = queue.push(
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
        let id2 = queue.push(
            JobPriority::Visible,
            JobType::RenderTile {
                page_index: 0,
                tile_x: 1,
                tile_y: 0,
                zoom_level: 100,
                rotation: 0,
                is_preview: true,
            },
        );
        let id3 = queue.push(
            JobPriority::Visible,
            JobType::RenderTile {
                page_index: 0,
                tile_x: 2,
                tile_y: 0,
                zoom_level: 100,
                rotation: 0,
                is_preview: true,
            },
        );

        // Should come out in FIFO order
        assert_eq!(queue.pop().unwrap().id, id1);
        assert_eq!(queue.pop().unwrap().id, id2);
        assert_eq!(queue.pop().unwrap().id, id3);
    }

    #[test]
    fn test_peek() {
        let queue = PriorityQueue::new();
        assert!(queue.peek().is_none());

        let id1 = queue.push(
            JobPriority::Visible,
            JobType::LoadFile {
                path: PathBuf::from("test.pdf"),
            },
        );
        queue.push(JobPriority::Ocr, JobType::RunOcr { page_index: 0 });

        let peeked = queue.peek().unwrap();
        assert_eq!(peeked.id, id1);
        assert_eq!(peeked.priority, JobPriority::Visible);

        // Peek shouldn't remove the job
        assert_eq!(queue.len(), 2);
    }

    #[test]
    fn test_clear() {
        let queue = PriorityQueue::new();

        queue.push(
            JobPriority::Visible,
            JobType::LoadFile {
                path: PathBuf::from("test.pdf"),
            },
        );
        queue.push(JobPriority::Ocr, JobType::RunOcr { page_index: 0 });
        assert_eq!(queue.len(), 2);

        queue.clear();
        assert!(queue.is_empty());
        assert_eq!(queue.len(), 0);
    }

    #[test]
    fn test_remove_if() {
        let queue = PriorityQueue::new();

        queue.push(JobPriority::Visible, JobType::RunOcr { page_index: 0 });
        queue.push(JobPriority::Visible, JobType::RunOcr { page_index: 1 });
        queue.push(
            JobPriority::Margin,
            JobType::LoadFile {
                path: PathBuf::from("test.pdf"),
            },
        );
        assert_eq!(queue.len(), 3);

        // Remove all OCR jobs
        let removed = queue.remove_if(|job| matches!(job.job_type, JobType::RunOcr { .. }));
        assert_eq!(removed, 2);
        assert_eq!(queue.len(), 1);

        // Verify the remaining job is the LoadFile job
        let remaining = queue.pop().unwrap();
        assert!(matches!(remaining.job_type, JobType::LoadFile { .. }));
    }

    #[test]
    fn test_jobs_inspection() {
        let queue = PriorityQueue::new();

        queue.push(
            JobPriority::Visible,
            JobType::LoadFile {
                path: PathBuf::from("test.pdf"),
            },
        );
        queue.push(JobPriority::Ocr, JobType::RunOcr { page_index: 0 });

        let jobs = queue.jobs();
        assert_eq!(jobs.len(), 2);

        // Jobs should be present (order not guaranteed)
        assert!(jobs.iter().any(|j| j.priority == JobPriority::Visible));
        assert!(jobs.iter().any(|j| j.priority == JobPriority::Ocr));
    }

    #[test]
    fn test_default() {
        let queue = PriorityQueue::default();
        assert!(queue.is_empty());
    }

    #[test]
    fn test_mixed_priority_fifo() {
        let queue = PriorityQueue::new();

        // Insert jobs with mixed priorities
        let id1 = queue.push(
            JobPriority::Visible,
            JobType::LoadFile {
                path: PathBuf::from("1.pdf"),
            },
        );
        let id2 = queue.push(
            JobPriority::Margin,
            JobType::LoadFile {
                path: PathBuf::from("2.pdf"),
            },
        );
        let id3 = queue.push(
            JobPriority::Visible,
            JobType::LoadFile {
                path: PathBuf::from("3.pdf"),
            },
        );
        let id4 = queue.push(
            JobPriority::Margin,
            JobType::LoadFile {
                path: PathBuf::from("4.pdf"),
            },
        );
        let id5 = queue.push(
            JobPriority::Ocr,
            JobType::LoadFile {
                path: PathBuf::from("5.pdf"),
            },
        );

        // Should get: Visible (id1, id3 in FIFO), then Margin (id2, id4 in FIFO), then Ocr (id5)
        assert_eq!(queue.pop().unwrap().id, id1);
        assert_eq!(queue.pop().unwrap().id, id3);
        assert_eq!(queue.pop().unwrap().id, id2);
        assert_eq!(queue.pop().unwrap().id, id4);
        assert_eq!(queue.pop().unwrap().id, id5);
    }

    #[test]
    fn test_extract_text_job_type() {
        let queue = PriorityQueue::new();

        // Test ExtractText job type can be queued
        let id = queue.push(
            JobPriority::Adjacent,
            JobType::ExtractText { page_index: 5 },
        );

        let job = queue.pop().unwrap();
        assert_eq!(job.id, id);
        assert_eq!(job.priority, JobPriority::Adjacent);
        assert!(matches!(job.job_type, JobType::ExtractText { page_index: 5 }));
    }

    #[test]
    fn test_remove_extract_text_jobs() {
        let queue = PriorityQueue::new();

        queue.push(JobPriority::Visible, JobType::ExtractText { page_index: 0 });
        queue.push(JobPriority::Visible, JobType::ExtractText { page_index: 1 });
        queue.push(JobPriority::Visible, JobType::RunOcr { page_index: 0 });
        assert_eq!(queue.len(), 3);

        // Remove all ExtractText jobs
        let removed = queue.remove_if(|job| matches!(job.job_type, JobType::ExtractText { .. }));
        assert_eq!(removed, 2);
        assert_eq!(queue.len(), 1);

        // Remaining job should be OCR
        let remaining = queue.pop().unwrap();
        assert!(matches!(remaining.job_type, JobType::RunOcr { .. }));
    }
}
