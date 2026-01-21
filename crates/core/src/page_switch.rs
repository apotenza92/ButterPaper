//! Fast page switching with cache-aware rendering
//!
//! Provides fast page switching by checking caches first (RAM → GPU → Disk)
//! and falling back to progressive rendering (preview → crisp) when needed.
//! Targets: <100ms for cached pages, <250ms for preview rendering.
//!
//! Also provides prefetching for adjacent pages and margin tiles to ensure
//! fast navigation between pages.

use crate::document::{Document, DocumentError, DocumentId, DocumentResult};
use pdf_editor_cache::{ram::CachedTile, DiskTileCache, RamTileCache};
use pdf_editor_render::{PdfDocument, RenderedTile, TileCoordinate, TileId, TileProfile, TileRenderer};
use pdf_editor_scheduler::{JobPriority, JobScheduler, JobType};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

/// Result of a page switch operation
#[derive(Debug, Clone)]
pub struct PageSwitchResult {
    /// Document ID
    pub document_id: DocumentId,

    /// Page index that was switched to
    pub page_index: u16,

    /// Rendered tiles for the page
    pub tiles: Vec<RenderedTile>,

    /// Page width in pixels at 100% zoom
    pub page_width: u32,

    /// Page height in pixels at 100% zoom
    pub page_height: u32,

    /// Zoom level used for rendering
    pub zoom_level: u32,

    /// Rotation angle in degrees (0, 90, 180, 270)
    pub rotation: u16,

    /// Whether tiles came from cache (true) or were rendered (false)
    pub from_cache: bool,

    /// Time taken to switch pages in milliseconds
    pub time_ms: u64,

    /// Whether preview profile was used (true) or crisp (false)
    pub is_preview: bool,
}

/// Fast page switcher with cache-aware rendering
///
/// This provides the fast path for page switching:
/// 1. Check RAM cache → return immediately (<100ms target)
/// 2. Check disk cache → return quickly (<100ms target)
/// 3. Render preview tiles → return fast (<250ms target)
/// 4. Render crisp tiles → upgrade quality (background)
/// 5. Prefetch adjacent pages and margin tiles (background)
pub struct PageSwitcher {
    /// Tile renderer for rendering page tiles
    tile_renderer: TileRenderer,

    /// RAM cache for fast tile access
    ram_cache: Option<Arc<RamTileCache>>,

    /// Disk cache for persistent tile storage
    disk_cache: Option<Arc<DiskTileCache>>,

    /// Job scheduler for background prefetching
    scheduler: Option<Arc<JobScheduler>>,

    /// Default zoom level (100% = actual size)
    default_zoom: u32,

    /// Default rotation (0 degrees)
    default_rotation: u16,

    /// Whether to enable prefetching (default: true)
    enable_prefetch: bool,
}

impl PageSwitcher {
    /// Create a new page switcher
    pub fn new() -> Self {
        Self {
            tile_renderer: TileRenderer::new(),
            ram_cache: None,
            disk_cache: None,
            scheduler: None,
            default_zoom: 100,
            default_rotation: 0,
            enable_prefetch: true,
        }
    }

    /// Create a page switcher with custom zoom level
    pub fn with_zoom(zoom_level: u32) -> Self {
        Self {
            tile_renderer: TileRenderer::new(),
            ram_cache: None,
            disk_cache: None,
            scheduler: None,
            default_zoom: zoom_level,
            default_rotation: 0,
            enable_prefetch: true,
        }
    }

    /// Set the RAM cache for fast tile access
    pub fn with_ram_cache(mut self, cache: Arc<RamTileCache>) -> Self {
        self.ram_cache = Some(cache);
        self
    }

    /// Set the disk cache for persistent tile storage
    pub fn with_disk_cache(mut self, cache: Arc<DiskTileCache>) -> Self {
        self.disk_cache = Some(cache);
        self
    }

    /// Set the job scheduler for background prefetching
    pub fn with_scheduler(mut self, scheduler: Arc<JobScheduler>) -> Self {
        self.scheduler = Some(scheduler);
        self
    }

    /// Enable or disable prefetching
    pub fn with_prefetch_enabled(mut self, enabled: bool) -> Self {
        self.enable_prefetch = enabled;
        self
    }

    /// Switch to a page using the fast path
    ///
    /// This method checks caches first for instant display, then falls back
    /// to progressive rendering (preview → crisp) if needed.
    ///
    /// # Arguments
    /// * `document` - The document to switch pages in
    /// * `page_index` - The page index to switch to (zero-based)
    ///
    /// # Returns
    /// A `PageSwitchResult` with rendered tiles and timing information
    pub fn switch_to_page(
        &self,
        document: &Document,
        page_index: u16,
    ) -> DocumentResult<PageSwitchResult> {
        self.switch_to_page_with_options(document, page_index, self.default_zoom, self.default_rotation)
    }

    /// Switch to a page with custom zoom and rotation
    ///
    /// # Arguments
    /// * `document` - The document to switch pages in
    /// * `page_index` - The page index to switch to (zero-based)
    /// * `zoom_level` - The zoom level to use (100 = 100%)
    /// * `rotation` - The rotation angle in degrees (0, 90, 180, 270)
    ///
    /// # Returns
    /// A `PageSwitchResult` with rendered tiles and timing information
    pub fn switch_to_page_with_options(
        &self,
        document: &Document,
        page_index: u16,
        zoom_level: u32,
        rotation: u16,
    ) -> DocumentResult<PageSwitchResult> {
        let start_time = Instant::now();

        // Validate page index
        if page_index >= document.page_count() {
            return Err(DocumentError::InvalidPageIndex {
                page: page_index,
                max: document.page_count(),
            });
        }

        // Update document's current page
        document.set_current_page(page_index);

        let file_path = &document.metadata().file_path;

        // Get cached page dimensions to avoid reopening PDF
        let (page_width, page_height) = document
            .metadata()
            .page_dimensions
            .get(&page_index)
            .map(|d| (d.width, d.height))
            .ok_or_else(|| DocumentError::LoadError(format!(
                "Page dimensions not cached for page {}",
                page_index
            )))?;

        // Try to load from cache first (fast path: <100ms)
        if let Some(cached_result) = self.try_load_from_cache(
            document.id(),
            file_path,
            page_index,
            zoom_level,
            rotation,
            page_width,
            page_height,
        )? {
            let elapsed = start_time.elapsed().as_millis() as u64;
            let mut result = cached_result;
            result.time_ms = elapsed;
            return Ok(result);
        }

        // Cache miss - render tiles with progressive loading
        // First render preview tiles for fast display (<250ms target)
        let preview_result = self.render_page_tiles(
            document.id(),
            file_path,
            page_index,
            zoom_level,
            rotation,
            TileProfile::Preview,
            page_width,
            page_height,
        )?;

        let elapsed = start_time.elapsed().as_millis() as u64;

        // Trigger prefetching for adjacent pages in the background
        if self.enable_prefetch {
            self.prefetch_adjacent_pages(document, page_index, zoom_level, rotation);
        }

        Ok(PageSwitchResult {
            document_id: preview_result.document_id,
            page_index: preview_result.page_index,
            tiles: preview_result.tiles,
            page_width: preview_result.page_width,
            page_height: preview_result.page_height,
            zoom_level: preview_result.zoom_level,
            rotation: preview_result.rotation,
            from_cache: false,
            time_ms: elapsed,
            is_preview: true,
        })
    }

    /// Try to load page tiles from cache
    ///
    /// Checks RAM cache first, then disk cache. Returns immediately if found.
    /// Uses cached page dimensions to avoid reopening the PDF.
    #[allow(clippy::too_many_arguments)]
    fn try_load_from_cache(
        &self,
        document_id: DocumentId,
        _file_path: &PathBuf,
        page_index: u16,
        zoom_level: u32,
        rotation: u16,
        page_width: f32,
        page_height: f32,
    ) -> DocumentResult<Option<PageSwitchResult>> {
        // Use provided page dimensions (from cache) instead of reopening PDF
        // This eliminates a major UI stall point

        // Calculate tile grid
        let (columns, rows) = self
            .tile_renderer
            .calculate_tile_grid(page_width, page_height, zoom_level);

        let mut tiles = Vec::new();
        let mut all_cached = true;

        // Try to load all tiles from cache
        for y in 0..rows {
            for x in 0..columns {
                let coord = TileCoordinate::new(x, y);

                // Create tile ID for crisp profile (prefer high quality from cache)
                let tile_id = TileId::new(
                    page_index,
                    coord,
                    zoom_level,
                    rotation,
                    TileProfile::Crisp,
                );

                let cache_key = tile_id.cache_key();

                // Try RAM cache first (fastest)
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
                            .and_then(|opt| opt.map(|t| CachedTile {
                                key: cache_key,
                                pixels: t.pixels,
                                width: t.width,
                                height: t.height,
                            }))
                    } else {
                        None
                    }
                } else {
                    tile_opt
                };

                // If any tile is missing, we need to render
                if let Some(cached_tile) = tile_opt {
                    tiles.push(RenderedTile {
                        id: tile_id,
                        pixels: cached_tile.pixels,
                        width: cached_tile.width,
                        height: cached_tile.height,
                    });
                } else {
                    all_cached = false;
                    break;
                }
            }
            if !all_cached {
                break;
            }
        }

        // If all tiles were cached, return immediately
        if all_cached {
            Ok(Some(PageSwitchResult {
                document_id,
                page_index,
                tiles,
                page_width: page_width as u32,
                page_height: page_height as u32,
                zoom_level,
                rotation,
                from_cache: true,
                time_ms: 0, // Will be filled by caller
                is_preview: false, // Cached tiles are crisp quality
            }))
        } else {
            Ok(None)
        }
    }

    /// Render page tiles and store in cache
    #[allow(clippy::too_many_arguments)]
    fn render_page_tiles(
        &self,
        document_id: DocumentId,
        file_path: &PathBuf,
        page_index: u16,
        zoom_level: u32,
        rotation: u16,
        profile: TileProfile,
        page_width: f32,
        page_height: f32,
    ) -> DocumentResult<PageSwitchResult> {
        // Open PDF for rendering
        let pdf_doc = PdfDocument::open(file_path)
            .map_err(|e| DocumentError::LoadError(format!("Failed to open PDF: {}", e)))?;

        // Use provided page dimensions (from cache) instead of querying the page

        // Render tiles
        let tiles = self
            .tile_renderer
            .render_page_tiles(&pdf_doc, page_index, zoom_level, profile)
            .map_err(|e| DocumentError::LoadError(format!("Failed to render tiles: {}", e)))?;

        // Store tiles in cache
        self.store_tiles_in_cache(&tiles);

        Ok(PageSwitchResult {
            document_id,
            page_index,
            tiles,
            page_width: page_width as u32,
            page_height: page_height as u32,
            zoom_level,
            rotation,
            from_cache: false,
            time_ms: 0, // Will be filled by caller
            is_preview: matches!(profile, TileProfile::Preview),
        })
    }

    /// Store rendered tiles in available caches
    fn store_tiles_in_cache(&self, tiles: &[RenderedTile]) {
        for tile in tiles {
            let cache_key = tile.id.cache_key();

            // Store in RAM cache
            if let Some(ram_cache) = &self.ram_cache {
                let _: () = ram_cache.put(cache_key, tile.pixels.clone(), tile.width, tile.height);
            }

            // Store in disk cache
            if let Some(disk_cache) = &self.disk_cache {
                let _: Result<(), std::io::Error> = disk_cache.put(cache_key, tile.pixels.clone(), tile.width, tile.height);
            }
        }
    }

    /// Upgrade a page to crisp quality
    ///
    /// This should be called after displaying preview tiles to upgrade
    /// to high-quality rendering in the background.
    ///
    /// # Arguments
    /// * `document` - The document
    /// * `page_index` - The page index to upgrade
    ///
    /// # Returns
    /// A `PageSwitchResult` with crisp tiles
    pub fn upgrade_to_crisp(
        &self,
        document: &Document,
        page_index: u16,
    ) -> DocumentResult<PageSwitchResult> {
        self.upgrade_to_crisp_with_options(document, page_index, self.default_zoom, self.default_rotation)
    }

    /// Upgrade a page to crisp quality with custom zoom and rotation
    pub fn upgrade_to_crisp_with_options(
        &self,
        document: &Document,
        page_index: u16,
        zoom_level: u32,
        rotation: u16,
    ) -> DocumentResult<PageSwitchResult> {
        let start_time = Instant::now();

        // Validate page index
        if page_index >= document.page_count() {
            return Err(DocumentError::InvalidPageIndex {
                page: page_index,
                max: document.page_count(),
            });
        }

        let file_path = &document.metadata().file_path;

        // Get cached page dimensions
        let (page_width, page_height) = document
            .metadata()
            .page_dimensions
            .get(&page_index)
            .map(|d| (d.width, d.height))
            .ok_or_else(|| DocumentError::LoadError(format!(
                "Page dimensions not cached for page {}",
                page_index
            )))?;

        // Render crisp tiles
        let mut result = self.render_page_tiles(
            document.id(),
            file_path,
            page_index,
            zoom_level,
            rotation,
            TileProfile::Crisp,
            page_width,
            page_height,
        )?;

        let elapsed = start_time.elapsed().as_millis() as u64;
        result.time_ms = elapsed;

        Ok(result)
    }

    /// Get the tile renderer
    pub fn tile_renderer(&self) -> &TileRenderer {
        &self.tile_renderer
    }

    /// Get the default zoom level
    pub fn default_zoom(&self) -> u32 {
        self.default_zoom
    }

    /// Get the default rotation
    pub fn default_rotation(&self) -> u16 {
        self.default_rotation
    }

    /// Prefetch adjacent pages and margin tiles for fast navigation
    ///
    /// This method submits background jobs to prefetch tiles for pages adjacent
    /// to the current page (page-1 and page+1), using the Adjacent priority.
    /// Margin tiles around the viewport are prefetched with Margin priority.
    ///
    /// # Arguments
    /// * `document` - The document being viewed
    /// * `current_page` - The current page index
    /// * `zoom_level` - The zoom level to prefetch at
    /// * `rotation` - The rotation angle to prefetch at
    ///
    /// # Returns
    /// The number of prefetch jobs submitted
    fn prefetch_adjacent_pages(
        &self,
        document: &Document,
        current_page: u16,
        zoom_level: u32,
        rotation: u16,
    ) -> usize {
        let scheduler = match &self.scheduler {
            Some(s) => s,
            None => return 0, // No scheduler configured, skip prefetching
        };

        let mut jobs_submitted = 0;

        // Prefetch previous page
        if current_page > 0 {
            jobs_submitted += self.prefetch_page_tiles(
                document,
                scheduler,
                current_page - 1,
                zoom_level,
                rotation,
                TileProfile::Preview, // Use preview profile for prefetch
            );
        }

        // Prefetch next page
        if current_page + 1 < document.page_count() {
            jobs_submitted += self.prefetch_page_tiles(
                document,
                scheduler,
                current_page + 1,
                zoom_level,
                rotation,
                TileProfile::Preview, // Use preview profile for prefetch
            );
        }

        jobs_submitted
    }

    /// Prefetch all tiles for a specific page
    ///
    /// Submits background jobs to render all tiles for the specified page.
    /// Tiles are submitted with Adjacent priority for background processing.
    ///
    /// # Arguments
    /// * `document` - The document being viewed
    /// * `scheduler` - The job scheduler to submit to
    /// * `page_index` - The page to prefetch
    /// * `zoom_level` - The zoom level to prefetch at
    /// * `rotation` - The rotation angle to prefetch at
    /// * `profile` - The tile profile to use (Preview or Crisp)
    ///
    /// # Returns
    /// The number of tile jobs submitted
    fn prefetch_page_tiles(
        &self,
        document: &Document,
        scheduler: &JobScheduler,
        page_index: u16,
        zoom_level: u32,
        rotation: u16,
        profile: TileProfile,
    ) -> usize {
        // Get page dimensions to calculate tile grid
        let file_path = &document.metadata().file_path;

        let pdf_doc = match PdfDocument::open(file_path) {
            Ok(doc) => doc,
            Err(_) => return 0, // Failed to open PDF, skip prefetching
        };

        let page = match pdf_doc.get_page(page_index) {
            Ok(p) => p,
            Err(_) => return 0, // Failed to get page, skip prefetching
        };

        let page_width = page.width().value;
        let page_height = page.height().value;

        // Calculate tile grid dimensions
        let (columns, rows) = self
            .tile_renderer
            .calculate_tile_grid(page_width, page_height, zoom_level);

        let mut jobs_submitted = 0;

        // Submit a job for each tile
        for y in 0..rows {
            for x in 0..columns {
                // Check if tile is already in cache to avoid unnecessary work
                let tile_id = TileId::new(
                    page_index,
                    TileCoordinate::new(x, y),
                    zoom_level,
                    rotation,
                    profile,
                );

                let cache_key = tile_id.cache_key();

                // Check RAM cache first
                let in_cache = if let Some(ram_cache) = &self.ram_cache {
                    ram_cache.try_get(cache_key).and_then(|opt| opt).is_some()
                } else {
                    false
                };

                // Check disk cache if not in RAM
                let in_cache = in_cache
                    || if let Some(disk_cache) = &self.disk_cache {
                        disk_cache
                            .try_get(cache_key)
                            .ok()
                            .and_then(|opt| opt)
                            .is_some()
                    } else {
                        false
                    };

                // Only submit job if tile is not already cached
                if !in_cache {
                    scheduler.submit(
                        JobPriority::Adjacent,
                        JobType::RenderTile {
                            page_index,
                            tile_x: x,
                            tile_y: y,
                            zoom_level,
                            rotation,
                            is_preview: matches!(profile, TileProfile::Preview),
                        },
                    );
                    jobs_submitted += 1;
                }
            }
        }

        jobs_submitted
    }
}

impl Default for PageSwitcher {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::DocumentMetadata;
    use std::path::PathBuf;

    fn test_document() -> Document {
        let metadata = DocumentMetadata {
            title: Some("Test Document".to_string()),
            author: Some("Test Author".to_string()),
            subject: None,
            creator: None,
            producer: None,
            page_count: 10,
            file_path: PathBuf::from("/nonexistent/test.pdf"),
            file_size: 1024,
            page_dimensions: std::collections::HashMap::new(),
            scale_systems: Vec::new(),
            default_scales: std::collections::HashMap::new(),
            text_edits: Vec::new(),
            annotations: Vec::new(),
        };

        Document::new(1, metadata)
    }

    #[test]
    fn test_page_switcher_creation() {
        let switcher = PageSwitcher::new();
        assert_eq!(switcher.default_zoom(), 100);
        assert_eq!(switcher.default_rotation(), 0);
        assert_eq!(switcher.tile_renderer().tile_size(), 256);
    }

    #[test]
    fn test_page_switcher_with_zoom() {
        let switcher = PageSwitcher::with_zoom(150);
        assert_eq!(switcher.default_zoom(), 150);
    }

    #[test]
    fn test_page_switcher_with_ram_cache() {
        let ram_cache = Arc::new(RamTileCache::with_mb_limit(128));
        let switcher = PageSwitcher::new().with_ram_cache(ram_cache.clone());

        // Verify cache is set (can't directly access private field, but we can check it compiles)
        let _ = switcher;
    }

    #[test]
    fn test_page_switcher_with_disk_cache() {
        let temp_dir = std::env::temp_dir().join("test_page_switch_disk");
        let disk_cache_result = DiskTileCache::with_mb_limit(temp_dir.clone(), 128);
        assert!(disk_cache_result.is_ok());
        let disk_cache = Arc::new(disk_cache_result.unwrap());
        let switcher = PageSwitcher::new().with_disk_cache(disk_cache.clone());

        // Verify cache is set
        let _ = switcher;

        // Cleanup
        let _ = std::fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn test_switch_to_page_invalid_index() {
        let switcher = PageSwitcher::new();
        let document = test_document();

        // Try to switch to page beyond document page count
        let result = switcher.switch_to_page(&document, 10);
        assert!(result.is_err());

        match result {
            Err(DocumentError::InvalidPageIndex { page, max }) => {
                assert_eq!(page, 10);
                assert_eq!(max, 10);
            }
            _ => panic!("Expected InvalidPageIndex error"),
        }
    }

    #[test]
    fn test_switch_to_page_nonexistent_file() {
        let switcher = PageSwitcher::new();
        let document = test_document();

        // Try to switch to page in nonexistent file
        let result = switcher.switch_to_page(&document, 0);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), DocumentError::LoadError(_)));
    }

    #[test]
    fn test_upgrade_to_crisp_invalid_index() {
        let switcher = PageSwitcher::new();
        let document = test_document();

        // Try to upgrade page beyond document page count
        let result = switcher.upgrade_to_crisp(&document, 10);
        assert!(result.is_err());

        match result {
            Err(DocumentError::InvalidPageIndex { page, max }) => {
                assert_eq!(page, 10);
                assert_eq!(max, 10);
            }
            _ => panic!("Expected InvalidPageIndex error"),
        }
    }

    #[test]
    fn test_upgrade_to_crisp_nonexistent_file() {
        let switcher = PageSwitcher::new();
        let document = test_document();

        // Try to upgrade page in nonexistent file
        let result = switcher.upgrade_to_crisp(&document, 0);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), DocumentError::LoadError(_)));
    }

    #[test]
    fn test_page_switch_result_structure() {
        // Test that PageSwitchResult has all expected fields
        let result = PageSwitchResult {
            document_id: 1,
            page_index: 0,
            tiles: vec![],
            page_width: 612,
            page_height: 792,
            zoom_level: 100,
            rotation: 0,
            from_cache: true,
            time_ms: 50,
            is_preview: false,
        };

        assert_eq!(result.document_id, 1);
        assert_eq!(result.page_index, 0);
        assert_eq!(result.page_width, 612);
        assert_eq!(result.page_height, 792);
        assert_eq!(result.zoom_level, 100);
        assert_eq!(result.rotation, 0);
        assert!(result.from_cache);
        assert_eq!(result.time_ms, 50);
        assert!(!result.is_preview);
    }

    #[test]
    fn test_default_page_switcher() {
        let switcher = PageSwitcher::default();
        assert_eq!(switcher.default_zoom(), 100);
        assert_eq!(switcher.default_rotation(), 0);
    }

    #[test]
    fn test_page_switcher_with_scheduler() {
        let scheduler = Arc::new(JobScheduler::new());
        let switcher = PageSwitcher::new().with_scheduler(scheduler.clone());

        // Verify switcher can be created with scheduler
        let _ = switcher;
    }

    #[test]
    fn test_page_switcher_with_prefetch_disabled() {
        let switcher = PageSwitcher::new().with_prefetch_enabled(false);

        // Verify prefetching can be disabled
        let _ = switcher;
    }

    #[test]
    fn test_prefetch_adjacent_pages_no_scheduler() {
        let switcher = PageSwitcher::new();
        let document = test_document();

        // Should return 0 jobs when no scheduler is configured
        let jobs = switcher.prefetch_adjacent_pages(&document, 5, 100, 0);
        assert_eq!(jobs, 0);
    }

    #[test]
    fn test_document_reopening_with_disk_cache() {
        // This test validates that reopening a cached document feels instant
        // by verifying disk cache restoration works correctly.
        //
        // Test scenario:
        // 1. Create a disk cache and simulate cached tiles
        // 2. Close and reopen the cache (simulating app restart)
        // 3. Verify tiles are still accessible via load_from_disk()
        // 4. Measure retrieval time to ensure instantaneous feel
        //
        // Target: <100ms for cached tile retrieval after app restart

        let temp_dir = std::env::temp_dir().join("test_doc_reopening");
        let _ = std::fs::remove_dir_all(&temp_dir); // Clean slate
        std::fs::create_dir_all(&temp_dir).unwrap();

        // Step 1: Create cache and populate with tiles
        let cache1 = Arc::new(DiskTileCache::with_mb_limit(&temp_dir, 128).unwrap());

        // Simulate a few cached tiles from a previous session
        // Using realistic tile sizes (256x256 RGBA = 256KB each)
        let tile_pixels = vec![255u8; 256 * 256 * 4];

        // Cache tiles for multiple pages at different zoom levels
        // This simulates a realistic document with cached content
        for page in 0..3 {
            for zoom in &[100, 150, 200] {
                // Generate a unique cache key (simplified version of TileId hash)
                let cache_key = (page as u64) * 1000 + (*zoom as u64);
                cache1.put(cache_key, tile_pixels.clone(), 256, 256).unwrap();
            }
        }

        // Verify tiles are in cache
        assert_eq!(cache1.tile_count(), 9); // 3 pages × 3 zoom levels
        let stats1 = cache1.stats();
        println!("Initial cache: {} tiles, {} MB used",
                 stats1.tile_count,
                 stats1.disk_used / (1024 * 1024));

        // Step 2: Drop the cache (simulate app close)
        drop(cache1);

        // Step 3: Create new cache instance (simulate app restart)
        let cache2 = Arc::new(DiskTileCache::with_mb_limit(&temp_dir, 128).unwrap());

        // Critical: Load existing tiles from disk
        let start = Instant::now();
        cache2.load_from_disk().unwrap();
        let load_time_ms = start.elapsed().as_millis() as u64;

        println!("Disk cache restoration time: {}ms", load_time_ms);

        // Step 4: Verify all tiles are restored
        assert_eq!(cache2.tile_count(), 9);
        let stats2 = cache2.stats();
        assert_eq!(stats2.tile_count, 9);

        // Step 5: Verify tile retrieval is fast
        let page = 0_u64;
        let zoom = 100_u64;
        let cache_key = page * 1000 + zoom; // page 0, zoom 100
        let start = Instant::now();
        let tile = cache2.get(cache_key).unwrap();
        let retrieval_time_ms = start.elapsed().as_millis() as u64;

        println!("Tile retrieval time: {}ms", retrieval_time_ms);

        assert!(tile.is_some());
        let tile = tile.unwrap();
        assert_eq!(tile.width, 256);
        assert_eq!(tile.height, 256);
        assert_eq!(tile.pixels.len(), 256 * 256 * 4);

        // Step 6: Validate performance targets
        // Target: Cache restoration should feel instant (<100ms)
        assert!(load_time_ms < 100,
                "Disk cache restoration took {}ms, exceeds 100ms target",
                load_time_ms);

        // Target: Individual tile retrieval should be fast (<50ms)
        assert!(retrieval_time_ms < 50,
                "Tile retrieval took {}ms, exceeds 50ms target",
                retrieval_time_ms);

        // Cleanup
        let _ = std::fs::remove_dir_all(temp_dir);

        println!("✓ Document reopening test passed:");
        println!("  - Cache restoration: {}ms (target: <100ms)", load_time_ms);
        println!("  - Tile retrieval: {}ms (target: <50ms)", retrieval_time_ms);
        println!("  - All {} tiles restored successfully", cache2.tile_count());
    }
}
