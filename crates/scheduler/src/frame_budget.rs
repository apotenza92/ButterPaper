//! Frame budget tracking for UI responsiveness
//!
//! Provides mechanisms to prevent UI freezes by tracking time spent on operations
//! and yielding control back to the event loop when budgets are exceeded.
//!
//! # Target Frame Times
//! - 120 FPS (ProMotion): 8.33ms per frame
//! - 60 FPS (standard): 16.67ms per frame
//!
//! The frame budget system ensures that heavy operations don't block the main
//! thread for too long, maintaining smooth animations and responsive input.

use std::time::{Duration, Instant};

/// Default frame budget for 60 FPS displays (16.67ms)
pub const FRAME_BUDGET_60FPS: Duration = Duration::from_micros(16_667);

/// Frame budget for 120 FPS displays (8.33ms)
pub const FRAME_BUDGET_120FPS: Duration = Duration::from_micros(8_333);

/// Minimum time to reserve for event processing (5ms)
pub const EVENT_PROCESSING_RESERVE: Duration = Duration::from_millis(5);

/// Frame budget tracker for preventing UI freezes
///
/// Tracks the time spent on operations within a frame and provides
/// checks to determine if the budget has been exceeded. Operations
/// should periodically check the budget and yield if necessary.
///
/// # Example
///
/// ```
/// use pdf_editor_scheduler::frame_budget::{FrameBudget, FRAME_BUDGET_60FPS};
/// use std::time::Duration;
///
/// let mut budget = FrameBudget::new(FRAME_BUDGET_60FPS);
///
/// // Simulate work
/// std::thread::sleep(Duration::from_millis(5));
///
/// // Check if we should yield
/// if budget.should_yield() {
///     // Yield to event loop
/// }
/// ```
#[derive(Debug, Clone)]
pub struct FrameBudget {
    /// When this frame started
    frame_start: Instant,

    /// Total budget for this frame
    budget: Duration,

    /// Time reserved for event processing
    reserved: Duration,

    /// Number of yield checks performed
    check_count: u32,
}

impl FrameBudget {
    /// Create a new frame budget tracker
    pub fn new(budget: Duration) -> Self {
        Self {
            frame_start: Instant::now(),
            budget,
            reserved: EVENT_PROCESSING_RESERVE,
            check_count: 0,
        }
    }

    /// Create a frame budget for 60 FPS displays
    pub fn for_60fps() -> Self {
        Self::new(FRAME_BUDGET_60FPS)
    }

    /// Create a frame budget for 120 FPS displays
    pub fn for_120fps() -> Self {
        Self::new(FRAME_BUDGET_120FPS)
    }

    /// Create a frame budget with custom reserve time
    pub fn with_reserved(mut self, reserved: Duration) -> Self {
        self.reserved = reserved;
        self
    }

    /// Reset the frame budget for a new frame
    pub fn reset(&mut self) {
        self.frame_start = Instant::now();
        self.check_count = 0;
    }

    /// Get the elapsed time since frame start
    pub fn elapsed(&self) -> Duration {
        self.frame_start.elapsed()
    }

    /// Get the remaining time in this frame's budget
    ///
    /// Returns `Duration::ZERO` if the budget has been exceeded.
    pub fn remaining(&self) -> Duration {
        let available = self.budget.saturating_sub(self.reserved);
        available.saturating_sub(self.elapsed())
    }

    /// Check if the frame budget has been exceeded
    ///
    /// Returns `true` if the remaining time is zero, indicating
    /// that the operation should yield to the event loop.
    pub fn is_exceeded(&self) -> bool {
        self.remaining() == Duration::ZERO
    }

    /// Check if we should yield to the event loop
    ///
    /// This is a convenience method that checks if the budget is exceeded
    /// and increments the check counter for statistics.
    pub fn should_yield(&mut self) -> bool {
        self.check_count += 1;
        self.is_exceeded()
    }

    /// Get the total frame budget
    pub fn budget(&self) -> Duration {
        self.budget
    }

    /// Get the reserved time
    pub fn reserved(&self) -> Duration {
        self.reserved
    }

    /// Get the number of yield checks performed
    pub fn check_count(&self) -> u32 {
        self.check_count
    }

    /// Get the available budget (total minus reserved)
    pub fn available_budget(&self) -> Duration {
        self.budget.saturating_sub(self.reserved)
    }
}

impl Default for FrameBudget {
    fn default() -> Self {
        Self::for_60fps()
    }
}

/// Operation progress tracker for chunked operations
///
/// Tracks progress through a large operation that needs to be split
/// across multiple frames to prevent UI freezes.
///
/// # Example
///
/// ```
/// use pdf_editor_scheduler::frame_budget::ChunkedOperation;
///
/// let mut op = ChunkedOperation::new(1000); // 1000 items to process
///
/// // Process a chunk
/// for _ in 0..100 {
///     // do work
///     op.advance(1);
/// }
///
/// assert_eq!(op.processed(), 100);
/// assert!(!op.is_complete());
/// ```
#[derive(Debug, Clone)]
pub struct ChunkedOperation {
    /// Total items to process
    total: u64,

    /// Items processed so far
    processed: u64,

    /// Target chunk size (items per frame)
    chunk_size: u64,

    /// Number of frames used
    frames_used: u32,
}

impl ChunkedOperation {
    /// Create a new chunked operation tracker
    pub fn new(total: u64) -> Self {
        Self {
            total,
            processed: 0,
            chunk_size: 100, // Default chunk size
            frames_used: 0,
        }
    }

    /// Set the target chunk size (items per frame)
    pub fn with_chunk_size(mut self, chunk_size: u64) -> Self {
        self.chunk_size = chunk_size;
        self
    }

    /// Get the total number of items
    pub fn total(&self) -> u64 {
        self.total
    }

    /// Get the number of items processed
    pub fn processed(&self) -> u64 {
        self.processed
    }

    /// Get the remaining items to process
    pub fn remaining(&self) -> u64 {
        self.total.saturating_sub(self.processed)
    }

    /// Check if the operation is complete
    pub fn is_complete(&self) -> bool {
        self.processed >= self.total
    }

    /// Get the progress as a percentage (0.0 to 100.0)
    pub fn progress_percent(&self) -> f32 {
        if self.total == 0 {
            return 100.0;
        }
        (self.processed as f32 / self.total as f32) * 100.0
    }

    /// Advance the processed count
    pub fn advance(&mut self, count: u64) {
        self.processed = self.processed.saturating_add(count).min(self.total);
    }

    /// Mark a frame as completed
    pub fn complete_frame(&mut self) {
        self.frames_used += 1;
    }

    /// Get the number of frames used
    pub fn frames_used(&self) -> u32 {
        self.frames_used
    }

    /// Get the target chunk size
    pub fn chunk_size(&self) -> u64 {
        self.chunk_size
    }

    /// Calculate items to process in current chunk
    ///
    /// Returns the minimum of the chunk size and remaining items.
    pub fn items_for_chunk(&self) -> u64 {
        self.remaining().min(self.chunk_size)
    }

    /// Get the start index for the current chunk
    pub fn chunk_start(&self) -> u64 {
        self.processed
    }

    /// Get the end index for the current chunk (exclusive)
    pub fn chunk_end(&self) -> u64 {
        (self.processed + self.chunk_size).min(self.total)
    }
}

/// Work yielder that checks budget and yields periodically
///
/// Useful for loop-based operations that need to periodically
/// check if they should yield control back to the event loop.
///
/// # Example
///
/// ```
/// use pdf_editor_scheduler::frame_budget::{WorkYielder, FrameBudget};
///
/// let budget = FrameBudget::for_60fps();
/// let mut yielder = WorkYielder::new(budget, 10); // Check every 10 iterations
///
/// for i in 0..100 {
///     // do work
///     if yielder.check_yield() {
///         // Budget exceeded, should yield
///         break;
///     }
/// }
/// ```
#[derive(Debug, Clone)]
pub struct WorkYielder {
    budget: FrameBudget,
    check_interval: u32,
    iterations: u32,
}

impl WorkYielder {
    /// Create a new work yielder
    pub fn new(budget: FrameBudget, check_interval: u32) -> Self {
        Self {
            budget,
            check_interval: check_interval.max(1),
            iterations: 0,
        }
    }

    /// Check if we should yield
    ///
    /// Only actually checks the budget every `check_interval` iterations
    /// to reduce overhead from calling `Instant::now()`.
    pub fn check_yield(&mut self) -> bool {
        self.iterations += 1;
        if self.iterations.is_multiple_of(self.check_interval) {
            self.budget.should_yield()
        } else {
            false
        }
    }

    /// Get the number of iterations performed
    pub fn iterations(&self) -> u32 {
        self.iterations
    }

    /// Get the frame budget
    pub fn budget(&self) -> &FrameBudget {
        &self.budget
    }

    /// Get the check interval
    pub fn check_interval(&self) -> u32 {
        self.check_interval
    }

    /// Reset for a new frame
    pub fn reset(&mut self) {
        self.budget.reset();
        self.iterations = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_frame_budget_creation() {
        let budget = FrameBudget::for_60fps();
        assert_eq!(budget.budget(), FRAME_BUDGET_60FPS);

        let budget = FrameBudget::for_120fps();
        assert_eq!(budget.budget(), FRAME_BUDGET_120FPS);

        let custom = FrameBudget::new(Duration::from_millis(10));
        assert_eq!(custom.budget(), Duration::from_millis(10));
    }

    #[test]
    fn test_frame_budget_with_reserved() {
        let budget = FrameBudget::for_60fps().with_reserved(Duration::from_millis(2));
        assert_eq!(budget.reserved(), Duration::from_millis(2));
    }

    #[test]
    fn test_frame_budget_remaining() {
        let budget = FrameBudget::new(Duration::from_millis(20))
            .with_reserved(Duration::from_millis(5));

        // Available is 15ms (20 - 5)
        assert_eq!(budget.available_budget(), Duration::from_millis(15));

        // Initially should have almost all available
        assert!(budget.remaining() > Duration::from_millis(14));
    }

    #[test]
    fn test_frame_budget_exceeded() {
        let budget = FrameBudget::new(Duration::from_millis(5))
            .with_reserved(Duration::from_millis(2));

        // Available is 3ms
        assert!(!budget.is_exceeded());

        // Sleep to exceed budget
        thread::sleep(Duration::from_millis(4));
        assert!(budget.is_exceeded());
    }

    #[test]
    fn test_frame_budget_reset() {
        let mut budget = FrameBudget::new(Duration::from_millis(10))
            .with_reserved(Duration::from_millis(2));

        thread::sleep(Duration::from_millis(5));
        let elapsed_before = budget.elapsed();
        assert!(elapsed_before >= Duration::from_millis(5));

        budget.reset();
        let elapsed_after = budget.elapsed();
        assert!(elapsed_after < Duration::from_millis(1));
    }

    #[test]
    fn test_frame_budget_should_yield() {
        let mut budget = FrameBudget::new(Duration::from_millis(2))
            .with_reserved(Duration::ZERO);

        assert_eq!(budget.check_count(), 0);
        let yielded1 = budget.should_yield();
        assert_eq!(budget.check_count(), 1);

        // Sleep to exceed budget
        thread::sleep(Duration::from_millis(3));
        let yielded2 = budget.should_yield();
        assert_eq!(budget.check_count(), 2);

        // First check should not yield, second should
        assert!(!yielded1 || yielded2); // At least one should be true
    }

    #[test]
    fn test_chunked_operation_basic() {
        let op = ChunkedOperation::new(1000);

        assert_eq!(op.total(), 1000);
        assert_eq!(op.processed(), 0);
        assert_eq!(op.remaining(), 1000);
        assert!(!op.is_complete());
        assert_eq!(op.progress_percent(), 0.0);
    }

    #[test]
    fn test_chunked_operation_advance() {
        let mut op = ChunkedOperation::new(100);

        op.advance(25);
        assert_eq!(op.processed(), 25);
        assert_eq!(op.remaining(), 75);
        assert_eq!(op.progress_percent(), 25.0);
        assert!(!op.is_complete());

        op.advance(75);
        assert_eq!(op.processed(), 100);
        assert_eq!(op.remaining(), 0);
        assert_eq!(op.progress_percent(), 100.0);
        assert!(op.is_complete());
    }

    #[test]
    fn test_chunked_operation_advance_overflow_protection() {
        let mut op = ChunkedOperation::new(100);

        op.advance(150); // Advance beyond total
        assert_eq!(op.processed(), 100); // Should be capped at total
        assert!(op.is_complete());
    }

    #[test]
    fn test_chunked_operation_with_chunk_size() {
        let op = ChunkedOperation::new(1000).with_chunk_size(50);

        assert_eq!(op.chunk_size(), 50);
        assert_eq!(op.items_for_chunk(), 50);
        assert_eq!(op.chunk_start(), 0);
        assert_eq!(op.chunk_end(), 50);
    }

    #[test]
    fn test_chunked_operation_chunk_boundaries() {
        let mut op = ChunkedOperation::new(100).with_chunk_size(30);

        // First chunk: 0-30
        assert_eq!(op.chunk_start(), 0);
        assert_eq!(op.chunk_end(), 30);
        assert_eq!(op.items_for_chunk(), 30);

        op.advance(30);

        // Second chunk: 30-60
        assert_eq!(op.chunk_start(), 30);
        assert_eq!(op.chunk_end(), 60);
        assert_eq!(op.items_for_chunk(), 30);

        op.advance(30);

        // Third chunk: 60-90
        assert_eq!(op.chunk_start(), 60);
        assert_eq!(op.chunk_end(), 90);

        op.advance(30);

        // Fourth chunk: 90-100 (partial)
        assert_eq!(op.chunk_start(), 90);
        assert_eq!(op.chunk_end(), 100);
        assert_eq!(op.items_for_chunk(), 10);
    }

    #[test]
    fn test_chunked_operation_frames_used() {
        let mut op = ChunkedOperation::new(100);

        assert_eq!(op.frames_used(), 0);
        op.complete_frame();
        assert_eq!(op.frames_used(), 1);
        op.complete_frame();
        assert_eq!(op.frames_used(), 2);
    }

    #[test]
    fn test_chunked_operation_zero_total() {
        let op = ChunkedOperation::new(0);

        assert!(op.is_complete());
        assert_eq!(op.progress_percent(), 100.0);
        assert_eq!(op.remaining(), 0);
    }

    #[test]
    fn test_work_yielder_basic() {
        let budget = FrameBudget::for_60fps();
        let yielder = WorkYielder::new(budget, 10);

        assert_eq!(yielder.iterations(), 0);
        assert_eq!(yielder.check_interval(), 10);
    }

    #[test]
    fn test_work_yielder_check_interval() {
        let budget = FrameBudget::new(Duration::from_secs(1)); // Long budget
        let mut yielder = WorkYielder::new(budget, 5);

        // Check every 5 iterations
        for i in 1..=20 {
            let should = yielder.check_yield();
            if i % 5 == 0 {
                // Actually checked budget
                // May or may not yield depending on time
            } else {
                // Did not check budget, should not yield
                assert!(!should);
            }
        }
        assert_eq!(yielder.iterations(), 20);
    }

    #[test]
    fn test_work_yielder_minimum_interval() {
        let budget = FrameBudget::for_60fps();
        let yielder = WorkYielder::new(budget, 0);

        // Interval should be at least 1
        assert_eq!(yielder.check_interval(), 1);
    }

    #[test]
    fn test_work_yielder_reset() {
        let budget = FrameBudget::for_60fps();
        let mut yielder = WorkYielder::new(budget, 5);

        for _ in 0..10 {
            yielder.check_yield();
        }
        assert_eq!(yielder.iterations(), 10);

        yielder.reset();
        assert_eq!(yielder.iterations(), 0);
    }

    #[test]
    fn test_frame_budget_default() {
        let budget = FrameBudget::default();
        assert_eq!(budget.budget(), FRAME_BUDGET_60FPS);
    }

    #[test]
    fn test_60fps_vs_120fps_budgets() {
        let budget_60 = FrameBudget::for_60fps();
        let budget_120 = FrameBudget::for_120fps();

        // 120fps budget should be half of 60fps
        assert!(budget_120.budget() < budget_60.budget());
        assert_eq!(budget_60.budget().as_micros(), 16_667);
        assert_eq!(budget_120.budget().as_micros(), 8_333);
    }

    #[test]
    fn test_budget_remaining_never_negative() {
        let budget = FrameBudget::new(Duration::from_millis(1))
            .with_reserved(Duration::ZERO);

        // Sleep way past budget
        thread::sleep(Duration::from_millis(10));

        // Remaining should be zero, not negative
        assert_eq!(budget.remaining(), Duration::ZERO);
        assert!(budget.is_exceeded());
    }

    #[test]
    fn test_frame_budget_elapsed_accuracy() {
        let budget = FrameBudget::new(Duration::from_secs(1));
        let start = budget.elapsed();

        thread::sleep(Duration::from_millis(10));

        let after = budget.elapsed();
        assert!(after > start);
        assert!(after >= Duration::from_millis(10));
    }
}
