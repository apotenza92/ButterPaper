//! Progressive page loading for large PDFs
//!
//! This module provides frame-budget-aware progressive page loading that prevents
//! UI freezes when loading pages from large PDF documents. It builds on the
//! existing progressive tile loading infrastructure to provide:
//!
//! - **Chunked tile loading**: Tiles are loaded in chunks that respect frame budgets
//! - **Progressive quality**: Preview tiles are loaded first, then upgraded to crisp
//! - **Background prefetching**: Adjacent pages are prefetched during idle time
//! - **Cancellation support**: Loading can be cancelled when viewport changes
//!
//! # Performance Targets
//! - First visible content: <100ms (show something immediately)
//! - Preview tiles complete: <250ms (fast visual feedback)
//! - Crisp upgrade: Background (no UI blocking)
//!
//! # Example
//!
//! ```ignore
//! use pdf_editor_core::progressive_page::{ProgressivePageLoader, LoadingState};
//! use pdf_editor_scheduler::FrameBudget;
//!
//! let loader = ProgressivePageLoader::new();
//! let mut state = loader.start_loading(document, page_index, zoom, rotation)?;
//!
//! // In render loop:
//! loop {
//!     let mut budget = FrameBudget::for_60fps();
//!
//!     // Load tiles within frame budget
//!     let progress = loader.load_chunk(&mut state, &mut budget)?;
//!
//!     // Render available tiles
//!     for tile in state.available_tiles() {
//!         render_tile(tile);
//!     }
//!
//!     if progress.is_complete() {
//!         break;
//!     }
//! }
//! ```

use crate::document::{Document, DocumentError, DocumentResult, DocumentId};
use pdf_editor_cache::{ram::CachedTile, DiskTileCache, RamTileCache};
use pdf_editor_render::{
    PdfDocument, RenderedTile, TileCoordinate, TileId, TileProfile, TileRenderer,
};
use pdf_editor_scheduler::{
    frame_budget::FrameBudget,
    JobPriority, JobScheduler, JobType,
};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Current stage of progressive loading
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadingStage {
    /// Not started yet
    NotStarted,

    /// Loading preview tiles (fast, lower quality)
    LoadingPreview,

    /// Preview complete, upgrading to crisp quality
    UpgradingToCrisp,

    /// All loading complete
    Complete,

    /// Loading was cancelled
    Cancelled,
}

/// Progress information for a loading operation
#[derive(Debug, Clone)]
pub struct LoadingProgress {
    /// Current loading stage
    pub stage: LoadingStage,

    /// Total tiles to load in current stage
    pub total_tiles: u32,

    /// Tiles loaded so far in current stage
    pub tiles_loaded: u32,

    /// Time elapsed since loading started
    pub elapsed: Duration,

    /// Whether loading is complete (all stages done)
    pub is_complete: bool,

    /// Whether any tiles are available for rendering
    pub has_renderable_content: bool,
}

impl LoadingProgress {
    /// Get progress percentage for current stage (0.0 to 100.0)
    pub fn stage_percent(&self) -> f32 {
        if self.total_tiles == 0 {
            return 100.0;
        }
        (self.tiles_loaded as f32 / self.total_tiles as f32) * 100.0
    }

    /// Check if current stage is complete
    pub fn stage_complete(&self) -> bool {
        self.tiles_loaded >= self.total_tiles
    }
}

/// State for an ongoing progressive page load operation
pub struct ProgressiveLoadState {
    /// Document being loaded
    document_id: DocumentId,

    /// Page being loaded
    page_index: u16,

    /// Zoom level
    zoom_level: u32,

    /// Rotation angle
    rotation: u16,

    /// Page dimensions
    page_width: f32,
    page_height: f32,

    /// Current loading stage
    stage: LoadingStage,

    /// Tiles loaded so far (preview quality)
    preview_tiles: Vec<RenderedTile>,

    /// Tiles loaded so far (crisp quality)
    crisp_tiles: Vec<RenderedTile>,

    /// Tile grid dimensions (columns, rows)
    grid_size: (u32, u32),

    /// Current tile index being loaded
    current_tile_index: u32,

    /// Total tiles in grid
    total_tiles: u32,

    /// When loading started
    start_time: Instant,

    /// PDF document handle (cached for reuse)
    pdf_document: Option<PdfDocument>,

    /// File path for the document
    file_path: PathBuf,

    /// Whether loading has been cancelled
    cancelled: bool,
}

impl ProgressiveLoadState {
    /// Get all tiles available for rendering (preview or crisp)
    pub fn available_tiles(&self) -> &[RenderedTile] {
        if !self.crisp_tiles.is_empty() {
            &self.crisp_tiles
        } else {
            &self.preview_tiles
        }
    }

    /// Get preview tiles only
    pub fn preview_tiles(&self) -> &[RenderedTile] {
        &self.preview_tiles
    }

    /// Get crisp tiles only
    pub fn crisp_tiles(&self) -> &[RenderedTile] {
        &self.crisp_tiles
    }

    /// Check if loading has been cancelled
    pub fn is_cancelled(&self) -> bool {
        self.cancelled
    }

    /// Get the current loading stage
    pub fn stage(&self) -> LoadingStage {
        self.stage
    }

    /// Get page index
    pub fn page_index(&self) -> u16 {
        self.page_index
    }

    /// Get document ID
    pub fn document_id(&self) -> DocumentId {
        self.document_id
    }

    /// Get elapsed time since loading started
    pub fn elapsed(&self) -> Duration {
        self.start_time.elapsed()
    }

    /// Get the page width in points
    pub fn page_width(&self) -> f32 {
        self.page_width
    }

    /// Get the page height in points
    pub fn page_height(&self) -> f32 {
        self.page_height
    }

    /// Get the zoom level
    pub fn zoom_level(&self) -> u32 {
        self.zoom_level
    }

    /// Get the rotation angle
    pub fn rotation(&self) -> u16 {
        self.rotation
    }
}

/// Progressive page loader with frame-budget awareness
///
/// Loads page tiles progressively within frame budgets to prevent UI freezes.
/// Supports preview → crisp quality progression and background prefetching.
pub struct ProgressivePageLoader {
    /// Tile renderer
    tile_renderer: TileRenderer,

    /// RAM cache for fast tile access
    ram_cache: Option<Arc<RamTileCache>>,

    /// Disk cache for persistent tile storage
    disk_cache: Option<Arc<DiskTileCache>>,

    /// Job scheduler for background work
    scheduler: Option<Arc<JobScheduler>>,

    /// Maximum tiles to load per chunk (within one frame budget check)
    tiles_per_chunk: u32,

    /// Whether to enable background prefetching
    enable_prefetch: bool,
}

impl ProgressivePageLoader {
    /// Create a new progressive page loader
    pub fn new() -> Self {
        Self {
            tile_renderer: TileRenderer::new(),
            ram_cache: None,
            disk_cache: None,
            scheduler: None,
            tiles_per_chunk: 4, // Load up to 4 tiles per budget check
            enable_prefetch: true,
        }
    }

    /// Set the RAM cache
    pub fn with_ram_cache(mut self, cache: Arc<RamTileCache>) -> Self {
        self.ram_cache = Some(cache);
        self
    }

    /// Set the disk cache
    pub fn with_disk_cache(mut self, cache: Arc<DiskTileCache>) -> Self {
        self.disk_cache = Some(cache);
        self
    }

    /// Set the job scheduler
    pub fn with_scheduler(mut self, scheduler: Arc<JobScheduler>) -> Self {
        self.scheduler = Some(scheduler);
        self
    }

    /// Set the tiles per chunk limit
    pub fn with_tiles_per_chunk(mut self, count: u32) -> Self {
        self.tiles_per_chunk = count.max(1);
        self
    }

    /// Enable or disable prefetching
    pub fn with_prefetch(mut self, enabled: bool) -> Self {
        self.enable_prefetch = enabled;
        self
    }

    /// Start loading a page progressively
    ///
    /// Returns a `ProgressiveLoadState` that tracks the loading progress.
    /// Call `load_chunk` repeatedly to load tiles within frame budgets.
    pub fn start_loading(
        &self,
        document: &Document,
        page_index: u16,
        zoom_level: u32,
        rotation: u16,
    ) -> DocumentResult<ProgressiveLoadState> {
        // Validate page index
        if page_index >= document.page_count() {
            return Err(DocumentError::InvalidPageIndex {
                page: page_index,
                max: document.page_count(),
            });
        }

        // Get page dimensions from cached metadata
        let (page_width, page_height) = document
            .metadata()
            .page_dimensions
            .get(&page_index)
            .map(|d| (d.width, d.height))
            .ok_or_else(|| {
                DocumentError::LoadError(format!(
                    "Page dimensions not cached for page {}",
                    page_index
                ))
            })?;

        // Calculate tile grid
        let (columns, rows) = self.tile_renderer.calculate_tile_grid(
            page_width,
            page_height,
            zoom_level,
        );
        let total_tiles = columns * rows;

        let file_path = document.metadata().file_path.clone();

        Ok(ProgressiveLoadState {
            document_id: document.id(),
            page_index,
            zoom_level,
            rotation,
            page_width,
            page_height,
            stage: LoadingStage::NotStarted,
            preview_tiles: Vec::with_capacity(total_tiles as usize),
            crisp_tiles: Vec::with_capacity(total_tiles as usize),
            grid_size: (columns, rows),
            current_tile_index: 0,
            total_tiles,
            start_time: Instant::now(),
            pdf_document: None,
            file_path,
            cancelled: false,
        })
    }

    /// Load a chunk of tiles within the given frame budget
    ///
    /// Returns progress information. Call this repeatedly until `is_complete` is true.
    pub fn load_chunk(
        &self,
        state: &mut ProgressiveLoadState,
        budget: &mut FrameBudget,
    ) -> DocumentResult<LoadingProgress> {
        if state.cancelled {
            return Ok(LoadingProgress {
                stage: LoadingStage::Cancelled,
                total_tiles: state.total_tiles,
                tiles_loaded: state.current_tile_index,
                elapsed: state.start_time.elapsed(),
                is_complete: true,
                has_renderable_content: !state.preview_tiles.is_empty(),
            });
        }

        // Transition from NotStarted to LoadingPreview
        if state.stage == LoadingStage::NotStarted {
            state.stage = LoadingStage::LoadingPreview;
            state.current_tile_index = 0;

            // Try to load from cache first
            if let Some(cached_result) = self.try_load_from_cache(state)? {
                state.crisp_tiles = cached_result;
                state.stage = LoadingStage::Complete;
                return Ok(LoadingProgress {
                    stage: LoadingStage::Complete,
                    total_tiles: state.total_tiles,
                    tiles_loaded: state.total_tiles,
                    elapsed: state.start_time.elapsed(),
                    is_complete: true,
                    has_renderable_content: true,
                });
            }

            // Open PDF document for rendering
            state.pdf_document = Some(
                PdfDocument::open(&state.file_path)
                    .map_err(|e| DocumentError::LoadError(format!("Failed to open PDF: {}", e)))?
            );
        }

        // Load preview tiles
        if state.stage == LoadingStage::LoadingPreview {
            self.load_preview_chunk(state, budget)?;

            // Check if preview stage is complete
            if state.current_tile_index >= state.total_tiles {
                state.stage = LoadingStage::UpgradingToCrisp;
                state.current_tile_index = 0;

                // Trigger prefetch for adjacent pages
                if self.enable_prefetch {
                    self.trigger_prefetch(state);
                }
            }

            return Ok(LoadingProgress {
                stage: state.stage,
                total_tiles: state.total_tiles,
                tiles_loaded: state.preview_tiles.len() as u32,
                elapsed: state.start_time.elapsed(),
                is_complete: false,
                has_renderable_content: !state.preview_tiles.is_empty(),
            });
        }

        // Upgrade to crisp tiles
        if state.stage == LoadingStage::UpgradingToCrisp {
            self.load_crisp_chunk(state, budget)?;

            // Check if crisp stage is complete
            if state.current_tile_index >= state.total_tiles {
                state.stage = LoadingStage::Complete;
                // Close PDF document to free resources
                state.pdf_document = None;
            }

            let is_complete = state.stage == LoadingStage::Complete;

            return Ok(LoadingProgress {
                stage: state.stage,
                total_tiles: state.total_tiles,
                tiles_loaded: state.crisp_tiles.len() as u32,
                elapsed: state.start_time.elapsed(),
                is_complete,
                has_renderable_content: !state.preview_tiles.is_empty() || !state.crisp_tiles.is_empty(),
            });
        }

        // Already complete
        Ok(LoadingProgress {
            stage: LoadingStage::Complete,
            total_tiles: state.total_tiles,
            tiles_loaded: state.total_tiles,
            elapsed: state.start_time.elapsed(),
            is_complete: true,
            has_renderable_content: !state.crisp_tiles.is_empty() || !state.preview_tiles.is_empty(),
        })
    }

    /// Cancel an ongoing loading operation
    pub fn cancel(&self, state: &mut ProgressiveLoadState) {
        state.cancelled = true;
        state.stage = LoadingStage::Cancelled;
        state.pdf_document = None; // Release PDF handle
    }

    /// Try to load all tiles from cache
    fn try_load_from_cache(
        &self,
        state: &ProgressiveLoadState,
    ) -> DocumentResult<Option<Vec<RenderedTile>>> {
        let (columns, rows) = state.grid_size;
        let mut tiles = Vec::with_capacity(state.total_tiles as usize);

        for y in 0..rows {
            for x in 0..columns {
                let coord = TileCoordinate::new(x, y);
                let tile_id = TileId::new(
                    state.page_index,
                    coord,
                    state.zoom_level,
                    state.rotation,
                    TileProfile::Crisp,
                );
                let cache_key = tile_id.cache_key();

                // Try RAM cache first
                let tile_opt: Option<CachedTile> = if let Some(ram_cache) = &self.ram_cache {
                    ram_cache.try_get(cache_key).and_then(|opt| opt)
                } else {
                    None
                };

                // Try disk cache if not in RAM
                let tile_opt = if tile_opt.is_none() {
                    if let Some(disk_cache) = &self.disk_cache {
                        disk_cache
                            .try_get(cache_key)
                            .ok()
                            .and_then(|opt| opt)
                            .and_then(|opt| {
                                opt.map(|t| CachedTile {
                                    key: cache_key,
                                    pixels: t.pixels,
                                    width: t.width,
                                    height: t.height,
                                })
                            })
                    } else {
                        None
                    }
                } else {
                    tile_opt
                };

                if let Some(cached_tile) = tile_opt {
                    tiles.push(RenderedTile {
                        id: tile_id,
                        pixels: cached_tile.pixels,
                        width: cached_tile.width,
                        height: cached_tile.height,
                    });
                } else {
                    // Cache miss - return None to trigger rendering
                    return Ok(None);
                }
            }
        }

        Ok(Some(tiles))
    }

    /// Load preview tiles within frame budget
    fn load_preview_chunk(
        &self,
        state: &mut ProgressiveLoadState,
        budget: &mut FrameBudget,
    ) -> DocumentResult<()> {
        let pdf_doc = state.pdf_document.as_ref().ok_or_else(|| {
            DocumentError::LoadError("PDF document not open".to_string())
        })?;

        let (columns, _rows) = state.grid_size;
        let mut tiles_loaded = 0;

        while state.current_tile_index < state.total_tiles
            && tiles_loaded < self.tiles_per_chunk
            && !budget.should_yield()
        {
            let tile_idx = state.current_tile_index;
            let x = tile_idx % columns;
            let y = tile_idx / columns;

            let coord = TileCoordinate::new(x, y);
            let tile_id = TileId::new(
                state.page_index,
                coord,
                state.zoom_level,
                state.rotation,
                TileProfile::Preview,
            );

            // Render preview tile
            let tile = self.tile_renderer.render_tile(pdf_doc, &tile_id)
                .map_err(|e| DocumentError::LoadError(format!("Failed to render tile: {}", e)))?;

            // Store in cache
            self.store_tile_in_cache(&tile);

            state.preview_tiles.push(tile);
            state.current_tile_index += 1;
            tiles_loaded += 1;
        }

        Ok(())
    }

    /// Load crisp tiles within frame budget
    fn load_crisp_chunk(
        &self,
        state: &mut ProgressiveLoadState,
        budget: &mut FrameBudget,
    ) -> DocumentResult<()> {
        let pdf_doc = state.pdf_document.as_ref().ok_or_else(|| {
            DocumentError::LoadError("PDF document not open".to_string())
        })?;

        let (columns, _rows) = state.grid_size;
        let mut tiles_loaded = 0;

        while state.current_tile_index < state.total_tiles
            && tiles_loaded < self.tiles_per_chunk
            && !budget.should_yield()
        {
            let tile_idx = state.current_tile_index;
            let x = tile_idx % columns;
            let y = tile_idx / columns;

            let coord = TileCoordinate::new(x, y);
            let tile_id = TileId::new(
                state.page_index,
                coord,
                state.zoom_level,
                state.rotation,
                TileProfile::Crisp,
            );

            // Render crisp tile
            let tile = self.tile_renderer.render_tile(pdf_doc, &tile_id)
                .map_err(|e| DocumentError::LoadError(format!("Failed to render tile: {}", e)))?;

            // Store in cache
            self.store_tile_in_cache(&tile);

            state.crisp_tiles.push(tile);
            state.current_tile_index += 1;
            tiles_loaded += 1;
        }

        Ok(())
    }

    /// Store a tile in available caches
    fn store_tile_in_cache(&self, tile: &RenderedTile) {
        let cache_key = tile.id.cache_key();

        if let Some(ram_cache) = &self.ram_cache {
            let _: () = ram_cache.put(cache_key, tile.pixels.clone(), tile.width, tile.height);
        }

        if let Some(disk_cache) = &self.disk_cache {
            let _: Result<(), std::io::Error> =
                disk_cache.put(cache_key, tile.pixels.clone(), tile.width, tile.height);
        }
    }

    /// Trigger background prefetch for adjacent pages
    fn trigger_prefetch(&self, state: &ProgressiveLoadState) {
        let scheduler = match &self.scheduler {
            Some(s) => s,
            None => return,
        };

        // Prefetch previous page
        if state.page_index > 0 {
            self.submit_prefetch_jobs(
                scheduler,
                state.page_index - 1,
                state.zoom_level,
                state.rotation,
                state.grid_size,
            );
        }

        // Prefetch next page (we don't know total pages here, scheduler will handle invalid pages)
        self.submit_prefetch_jobs(
            scheduler,
            state.page_index + 1,
            state.zoom_level,
            state.rotation,
            state.grid_size,
        );
    }

    /// Submit prefetch jobs for a page
    fn submit_prefetch_jobs(
        &self,
        scheduler: &JobScheduler,
        page_index: u16,
        zoom_level: u32,
        rotation: u16,
        grid_size: (u32, u32),
    ) {
        let (columns, rows) = grid_size;

        for y in 0..rows {
            for x in 0..columns {
                scheduler.submit(
                    JobPriority::Adjacent,
                    JobType::RenderTile {
                        page_index,
                        tile_x: x,
                        tile_y: y,
                        zoom_level,
                        rotation,
                        is_preview: true,
                    },
                );
            }
        }
    }

    /// Get the tile renderer
    pub fn tile_renderer(&self) -> &TileRenderer {
        &self.tile_renderer
    }
}

impl Default for ProgressivePageLoader {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::DocumentMetadata;
    use std::collections::HashMap;

    fn test_document_with_dimensions() -> Document {
        let mut page_dimensions = HashMap::new();
        page_dimensions.insert(0, crate::document::PageDimensions {
            width: 612.0,
            height: 792.0,
        });
        page_dimensions.insert(1, crate::document::PageDimensions {
            width: 612.0,
            height: 792.0,
        });
        page_dimensions.insert(2, crate::document::PageDimensions {
            width: 612.0,
            height: 792.0,
        });

        let metadata = DocumentMetadata {
            title: Some("Test Document".to_string()),
            author: None,
            subject: None,
            creator: None,
            producer: None,
            page_count: 3,
            file_path: PathBuf::from("/nonexistent/test.pdf"),
            file_size: 1024,
            page_dimensions,
            scale_systems: Vec::new(),
            default_scales: HashMap::new(),
            text_edits: Vec::new(),
            annotations: Vec::new(),
            measurements: Vec::new(),
        };

        Document::new(1, metadata)
    }

    #[test]
    fn test_progressive_loader_creation() {
        let loader = ProgressivePageLoader::new();
        assert_eq!(loader.tiles_per_chunk, 4);
        assert!(loader.enable_prefetch);
    }

    #[test]
    fn test_progressive_loader_builder() {
        let loader = ProgressivePageLoader::new()
            .with_tiles_per_chunk(8)
            .with_prefetch(false);

        assert_eq!(loader.tiles_per_chunk, 8);
        assert!(!loader.enable_prefetch);
    }

    #[test]
    fn test_start_loading_invalid_page() {
        let loader = ProgressivePageLoader::new();
        let document = test_document_with_dimensions();

        let result = loader.start_loading(&document, 10, 100, 0);
        assert!(result.is_err());

        match result {
            Err(DocumentError::InvalidPageIndex { page, max }) => {
                assert_eq!(page, 10);
                assert_eq!(max, 3);
            }
            _ => panic!("Expected InvalidPageIndex error"),
        }
    }

    #[test]
    fn test_start_loading_creates_state() {
        let loader = ProgressivePageLoader::new();
        let document = test_document_with_dimensions();

        let state = loader.start_loading(&document, 0, 100, 0).unwrap();

        assert_eq!(state.page_index(), 0);
        assert_eq!(state.document_id(), 1);
        assert_eq!(state.stage(), LoadingStage::NotStarted);
        assert!(!state.is_cancelled());
        assert!(state.available_tiles().is_empty());
    }

    #[test]
    fn test_cancel_loading() {
        let loader = ProgressivePageLoader::new();
        let document = test_document_with_dimensions();

        let mut state = loader.start_loading(&document, 0, 100, 0).unwrap();
        loader.cancel(&mut state);

        assert!(state.is_cancelled());
        assert_eq!(state.stage(), LoadingStage::Cancelled);
    }

    #[test]
    fn test_loading_progress_percentage() {
        let progress = LoadingProgress {
            stage: LoadingStage::LoadingPreview,
            total_tiles: 12,
            tiles_loaded: 6,
            elapsed: Duration::from_millis(100),
            is_complete: false,
            has_renderable_content: true,
        };

        assert_eq!(progress.stage_percent(), 50.0);
        assert!(!progress.stage_complete());
    }

    #[test]
    fn test_loading_progress_complete() {
        let progress = LoadingProgress {
            stage: LoadingStage::Complete,
            total_tiles: 12,
            tiles_loaded: 12,
            elapsed: Duration::from_millis(200),
            is_complete: true,
            has_renderable_content: true,
        };

        assert_eq!(progress.stage_percent(), 100.0);
        assert!(progress.stage_complete());
        assert!(progress.is_complete);
    }

    #[test]
    fn test_loading_progress_zero_tiles() {
        let progress = LoadingProgress {
            stage: LoadingStage::Complete,
            total_tiles: 0,
            tiles_loaded: 0,
            elapsed: Duration::ZERO,
            is_complete: true,
            has_renderable_content: false,
        };

        // Should handle zero tiles gracefully
        assert_eq!(progress.stage_percent(), 100.0);
        assert!(progress.stage_complete());
    }

    #[test]
    fn test_load_chunk_cancelled_state() {
        let loader = ProgressivePageLoader::new();
        let document = test_document_with_dimensions();

        let mut state = loader.start_loading(&document, 0, 100, 0).unwrap();
        loader.cancel(&mut state);

        let mut budget = FrameBudget::for_60fps();
        let progress = loader.load_chunk(&mut state, &mut budget).unwrap();

        assert_eq!(progress.stage, LoadingStage::Cancelled);
        assert!(progress.is_complete);
    }

    #[test]
    fn test_progressive_load_state_accessors() {
        let loader = ProgressivePageLoader::new();
        let document = test_document_with_dimensions();

        let state = loader.start_loading(&document, 1, 150, 90).unwrap();

        assert_eq!(state.page_index(), 1);
        assert_eq!(state.document_id(), 1);
        assert_eq!(state.stage(), LoadingStage::NotStarted);
        assert!(state.preview_tiles().is_empty());
        assert!(state.crisp_tiles().is_empty());
    }

    #[test]
    fn test_tiles_per_chunk_minimum() {
        // tiles_per_chunk should always be at least 1
        let loader = ProgressivePageLoader::new().with_tiles_per_chunk(0);
        assert_eq!(loader.tiles_per_chunk, 1);
    }

    #[test]
    fn test_loading_stage_transitions() {
        // Test that stages are correctly ordered
        assert_ne!(LoadingStage::NotStarted, LoadingStage::LoadingPreview);
        assert_ne!(LoadingStage::LoadingPreview, LoadingStage::UpgradingToCrisp);
        assert_ne!(LoadingStage::UpgradingToCrisp, LoadingStage::Complete);
        assert_ne!(LoadingStage::Complete, LoadingStage::Cancelled);
    }

    #[test]
    fn test_available_tiles_prefers_crisp() {
        let loader = ProgressivePageLoader::new();
        let document = test_document_with_dimensions();

        let mut state = loader.start_loading(&document, 0, 100, 0).unwrap();

        // Initially no tiles
        assert!(state.available_tiles().is_empty());

        // Add a preview tile
        state.preview_tiles.push(RenderedTile {
            id: TileId::new(0, TileCoordinate::new(0, 0), 100, 0, TileProfile::Preview),
            pixels: vec![0; 256 * 256 * 4],
            width: 256,
            height: 256,
        });

        // Should return preview tiles when no crisp tiles
        assert_eq!(state.available_tiles().len(), 1);
        assert_eq!(state.available_tiles()[0].id.profile, TileProfile::Preview);

        // Add a crisp tile
        state.crisp_tiles.push(RenderedTile {
            id: TileId::new(0, TileCoordinate::new(0, 0), 100, 0, TileProfile::Crisp),
            pixels: vec![0; 256 * 256 * 4],
            width: 256,
            height: 256,
        });

        // Should now return crisp tiles
        assert_eq!(state.available_tiles().len(), 1);
        assert_eq!(state.available_tiles()[0].id.profile, TileProfile::Crisp);
    }

    #[test]
    fn test_default_trait() {
        let loader = ProgressivePageLoader::default();
        assert_eq!(loader.tiles_per_chunk, 4);
    }
}
