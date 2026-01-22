//! Deferred background job scheduling for documents
//!
//! Provides a system for scheduling background jobs (OCR, thumbnails, indexing)
//! that run after the initial file open completes. This keeps file opening fast
//! by deferring non-critical work until the document is displayed.

use crate::document::DocumentId;

/// Types of deferred background jobs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DeferredJobType {
    /// Generate thumbnails for all pages
    Thumbnails,

    /// Run OCR on pages without selectable text
    Ocr,

    /// Build search index for document
    Indexing,
}

/// Configuration for deferred job scheduling
#[derive(Debug, Clone)]
pub struct DeferredJobConfig {
    /// Whether to generate thumbnails automatically
    pub enable_thumbnails: bool,

    /// Whether to run OCR automatically on pages without text
    pub enable_ocr: bool,

    /// Whether to build search index automatically
    pub enable_indexing: bool,

    /// Thumbnail width in pixels (height calculated from aspect ratio)
    pub thumbnail_width: u32,

    /// Thumbnail height in pixels
    pub thumbnail_height: u32,

    /// Whether to run deferred jobs immediately or wait until idle
    pub run_immediately: bool,
}

impl Default for DeferredJobConfig {
    fn default() -> Self {
        Self {
            enable_thumbnails: true,
            enable_ocr: true,
            enable_indexing: true,
            thumbnail_width: 150,
            thumbnail_height: 200,
            run_immediately: false, // Wait until document is displayed
        }
    }
}

impl DeferredJobConfig {
    /// Create a new deferred job configuration
    pub fn new() -> Self {
        Self::default()
    }

    /// Set whether to enable thumbnail generation
    pub fn with_thumbnails(mut self, enable: bool) -> Self {
        self.enable_thumbnails = enable;
        self
    }

    /// Set whether to enable OCR
    pub fn with_ocr(mut self, enable: bool) -> Self {
        self.enable_ocr = enable;
        self
    }

    /// Set whether to enable indexing
    pub fn with_indexing(mut self, enable: bool) -> Self {
        self.enable_indexing = enable;
        self
    }

    /// Set thumbnail dimensions
    pub fn with_thumbnail_size(mut self, width: u32, height: u32) -> Self {
        self.thumbnail_width = width;
        self.thumbnail_height = height;
        self
    }

    /// Set whether to run deferred jobs immediately
    pub fn with_run_immediately(mut self, run_immediately: bool) -> Self {
        self.run_immediately = run_immediately;
        self
    }
}

/// A deferred job for a document
#[derive(Debug, Clone)]
pub struct DeferredJob {
    /// Document ID this job is for
    pub document_id: DocumentId,

    /// Type of deferred job
    pub job_type: DeferredJobType,

    /// Page index (for per-page jobs like OCR)
    pub page_index: Option<u16>,
}

impl DeferredJob {
    /// Create a new deferred job
    pub fn new(document_id: DocumentId, job_type: DeferredJobType) -> Self {
        Self {
            document_id,
            job_type,
            page_index: None,
        }
    }

    /// Create a new deferred job for a specific page
    pub fn for_page(document_id: DocumentId, job_type: DeferredJobType, page_index: u16) -> Self {
        Self {
            document_id,
            job_type,
            page_index: Some(page_index),
        }
    }
}

/// Scheduler for deferred background jobs
///
/// This scheduler manages background work that should happen after
/// the initial document load. Jobs are scheduled with low priority
/// to avoid interfering with user interactions.
pub struct DeferredJobScheduler {
    config: DeferredJobConfig,
}

impl DeferredJobScheduler {
    /// Create a new deferred job scheduler
    pub fn new(config: DeferredJobConfig) -> Self {
        Self { config }
    }

    /// Schedule deferred jobs for a newly opened document
    ///
    /// This method should be called after a document is opened and
    /// the first page preview has been rendered. It schedules background
    /// jobs based on the configuration.
    ///
    /// Returns a list of deferred jobs that should be submitted to
    /// the main job scheduler.
    pub fn schedule_for_document(
        &self,
        document_id: DocumentId,
        page_count: u16,
    ) -> Vec<DeferredJob> {
        let mut jobs = Vec::new();

        // Schedule thumbnail generation for all pages
        if self.config.enable_thumbnails {
            for page_index in 0..page_count {
                jobs.push(DeferredJob::for_page(
                    document_id,
                    DeferredJobType::Thumbnails,
                    page_index,
                ));
            }
        }

        // Schedule OCR for all pages (will detect which pages need it)
        if self.config.enable_ocr {
            for page_index in 0..page_count {
                jobs.push(DeferredJob::for_page(
                    document_id,
                    DeferredJobType::Ocr,
                    page_index,
                ));
            }
        }

        // Schedule indexing (one job for entire document)
        if self.config.enable_indexing {
            jobs.push(DeferredJob::new(document_id, DeferredJobType::Indexing));
        }

        jobs
    }

    /// Schedule deferred jobs for the current page only
    ///
    /// This method schedules background jobs for a single page,
    /// useful when navigating to a new page. Higher priority than
    /// document-wide jobs.
    pub fn schedule_for_page(&self, document_id: DocumentId, page_index: u16) -> Vec<DeferredJob> {
        let mut jobs = Vec::new();

        // Schedule thumbnail for current page
        if self.config.enable_thumbnails {
            jobs.push(DeferredJob::for_page(
                document_id,
                DeferredJobType::Thumbnails,
                page_index,
            ));
        }

        // Schedule OCR for current page
        if self.config.enable_ocr {
            jobs.push(DeferredJob::for_page(
                document_id,
                DeferredJobType::Ocr,
                page_index,
            ));
        }

        jobs
    }

    /// Get the configuration
    pub fn config(&self) -> &DeferredJobConfig {
        &self.config
    }

    /// Get thumbnail dimensions from configuration
    pub fn thumbnail_size(&self) -> (u32, u32) {
        (self.config.thumbnail_width, self.config.thumbnail_height)
    }
}

impl Default for DeferredJobScheduler {
    fn default() -> Self {
        Self::new(DeferredJobConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deferred_job_config_default() {
        let config = DeferredJobConfig::default();
        assert!(config.enable_thumbnails);
        assert!(config.enable_ocr);
        assert!(config.enable_indexing);
        assert_eq!(config.thumbnail_width, 150);
        assert_eq!(config.thumbnail_height, 200);
        assert!(!config.run_immediately);
    }

    #[test]
    fn test_deferred_job_config_builder() {
        let config = DeferredJobConfig::new()
            .with_thumbnails(false)
            .with_ocr(true)
            .with_indexing(false)
            .with_thumbnail_size(100, 150)
            .with_run_immediately(true);

        assert!(!config.enable_thumbnails);
        assert!(config.enable_ocr);
        assert!(!config.enable_indexing);
        assert_eq!(config.thumbnail_width, 100);
        assert_eq!(config.thumbnail_height, 150);
        assert!(config.run_immediately);
    }

    #[test]
    fn test_deferred_job_creation() {
        let job = DeferredJob::new(123, DeferredJobType::Thumbnails);
        assert_eq!(job.document_id, 123);
        assert_eq!(job.job_type, DeferredJobType::Thumbnails);
        assert_eq!(job.page_index, None);
    }

    #[test]
    fn test_deferred_job_for_page() {
        let job = DeferredJob::for_page(456, DeferredJobType::Ocr, 5);
        assert_eq!(job.document_id, 456);
        assert_eq!(job.job_type, DeferredJobType::Ocr);
        assert_eq!(job.page_index, Some(5));
    }

    #[test]
    fn test_scheduler_creation() {
        let config = DeferredJobConfig::default();
        let scheduler = DeferredJobScheduler::new(config);
        assert!(scheduler.config().enable_thumbnails);
    }

    #[test]
    fn test_schedule_for_document_all_enabled() {
        let config = DeferredJobConfig::default();
        let scheduler = DeferredJobScheduler::new(config);

        let jobs = scheduler.schedule_for_document(100, 5);

        // Should schedule: 5 thumbnail jobs + 5 OCR jobs + 1 indexing job = 11 jobs
        assert_eq!(jobs.len(), 11);

        // Count job types
        let thumbnail_count = jobs
            .iter()
            .filter(|j| j.job_type == DeferredJobType::Thumbnails)
            .count();
        let ocr_count = jobs
            .iter()
            .filter(|j| j.job_type == DeferredJobType::Ocr)
            .count();
        let indexing_count = jobs
            .iter()
            .filter(|j| j.job_type == DeferredJobType::Indexing)
            .count();

        assert_eq!(thumbnail_count, 5);
        assert_eq!(ocr_count, 5);
        assert_eq!(indexing_count, 1);
    }

    #[test]
    fn test_schedule_for_document_thumbnails_only() {
        let config = DeferredJobConfig::new()
            .with_thumbnails(true)
            .with_ocr(false)
            .with_indexing(false);
        let scheduler = DeferredJobScheduler::new(config);

        let jobs = scheduler.schedule_for_document(100, 3);

        // Should schedule only 3 thumbnail jobs
        assert_eq!(jobs.len(), 3);
        assert!(jobs
            .iter()
            .all(|j| j.job_type == DeferredJobType::Thumbnails));
    }

    #[test]
    fn test_schedule_for_document_ocr_only() {
        let config = DeferredJobConfig::new()
            .with_thumbnails(false)
            .with_ocr(true)
            .with_indexing(false);
        let scheduler = DeferredJobScheduler::new(config);

        let jobs = scheduler.schedule_for_document(100, 2);

        // Should schedule only 2 OCR jobs
        assert_eq!(jobs.len(), 2);
        assert!(jobs.iter().all(|j| j.job_type == DeferredJobType::Ocr));
    }

    #[test]
    fn test_schedule_for_document_indexing_only() {
        let config = DeferredJobConfig::new()
            .with_thumbnails(false)
            .with_ocr(false)
            .with_indexing(true);
        let scheduler = DeferredJobScheduler::new(config);

        let jobs = scheduler.schedule_for_document(100, 5);

        // Should schedule only 1 indexing job
        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].job_type, DeferredJobType::Indexing);
        assert_eq!(jobs[0].page_index, None);
    }

    #[test]
    fn test_schedule_for_document_none_enabled() {
        let config = DeferredJobConfig::new()
            .with_thumbnails(false)
            .with_ocr(false)
            .with_indexing(false);
        let scheduler = DeferredJobScheduler::new(config);

        let jobs = scheduler.schedule_for_document(100, 5);

        // Should schedule no jobs
        assert_eq!(jobs.len(), 0);
    }

    #[test]
    fn test_schedule_for_page() {
        let config = DeferredJobConfig::default();
        let scheduler = DeferredJobScheduler::new(config);

        let jobs = scheduler.schedule_for_page(200, 3);

        // Should schedule 2 jobs: thumbnail + OCR for page 3
        assert_eq!(jobs.len(), 2);

        let thumbnail_count = jobs
            .iter()
            .filter(|j| j.job_type == DeferredJobType::Thumbnails)
            .count();
        let ocr_count = jobs
            .iter()
            .filter(|j| j.job_type == DeferredJobType::Ocr)
            .count();

        assert_eq!(thumbnail_count, 1);
        assert_eq!(ocr_count, 1);

        // Verify page index
        assert!(jobs.iter().all(|j| j.page_index == Some(3)));
        assert!(jobs.iter().all(|j| j.document_id == 200));
    }

    #[test]
    fn test_thumbnail_size() {
        let config = DeferredJobConfig::new().with_thumbnail_size(200, 300);
        let scheduler = DeferredJobScheduler::new(config);

        let (width, height) = scheduler.thumbnail_size();
        assert_eq!(width, 200);
        assert_eq!(height, 300);
    }

    #[test]
    fn test_default_scheduler() {
        let scheduler = DeferredJobScheduler::default();
        assert!(scheduler.config().enable_thumbnails);
        assert!(scheduler.config().enable_ocr);
        assert!(scheduler.config().enable_indexing);
    }
}
