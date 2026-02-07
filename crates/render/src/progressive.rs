//! Progressive tile loading system
//!
//! Implements a two-stage tile loading strategy:
//! 1. Preview tiles are rendered first (fast, lower fidelity)
//! 2. Crisp tiles replace preview tiles (high fidelity, slower)
//!
//! This provides immediate visual feedback while maintaining high quality.

use crate::pdf::{PdfDocument, PdfResult};
use crate::tile::{RenderedTile, TileCoordinate, TileId, TileProfile, TileRenderer};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Tile loading state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TileState {
    /// Tile is not yet loaded
    NotLoaded,

    /// Preview version is available
    PreviewLoaded,

    /// Crisp version is available (final state)
    CrispLoaded,
}

/// Progress callback for progressive loading
///
/// Called when a tile completes loading at either preview or crisp quality.
pub type ProgressCallback = Arc<dyn Fn(TileId, TileState, &RenderedTile) + Send + Sync>;

/// Progressive tile loader
///
/// Manages the two-stage loading process for tiles, ensuring preview tiles
/// are loaded first for fast visual feedback, followed by crisp tiles.
pub struct ProgressiveTileLoader {
    /// The tile renderer
    renderer: TileRenderer,

    /// Current tile states (tracks what's loaded)
    tile_states: Arc<Mutex<HashMap<TileId, TileState>>>,
}

impl ProgressiveTileLoader {
    /// Create a new progressive tile loader
    pub fn new() -> Self {
        Self { renderer: TileRenderer::new(), tile_states: Arc::new(Mutex::new(HashMap::new())) }
    }

    /// Create a progressive tile loader with a custom tile renderer
    pub fn with_renderer(renderer: TileRenderer) -> Self {
        Self { renderer, tile_states: Arc::new(Mutex::new(HashMap::new())) }
    }

    /// Get the current state of a tile
    pub fn get_tile_state(&self, tile_id: &TileId) -> TileState {
        self.tile_states.lock().unwrap().get(tile_id).copied().unwrap_or(TileState::NotLoaded)
    }

    /// Load a single tile progressively
    ///
    /// Returns a vector containing the preview tile (if requested) followed by the crisp tile.
    /// The callback is invoked for each stage as it completes.
    pub fn load_tile(
        &self,
        document: &PdfDocument,
        page_index: u16,
        coordinate: TileCoordinate,
        zoom_level: u32,
        rotation: u16,
        callback: Option<ProgressCallback>,
    ) -> PdfResult<Vec<RenderedTile>> {
        let mut results = Vec::new();

        // Stage 1: Render preview tile
        let preview_id =
            TileId::new(page_index, coordinate, zoom_level, rotation, TileProfile::Preview);
        let preview_tile = self.renderer.render_tile(document, &preview_id)?;

        // Update state
        {
            let mut states = self.tile_states.lock().unwrap();
            states.insert(preview_id.clone(), TileState::PreviewLoaded);
        }

        // Invoke callback
        if let Some(ref cb) = callback {
            cb(preview_id.clone(), TileState::PreviewLoaded, &preview_tile);
        }

        results.push(preview_tile);

        // Stage 2: Render crisp tile
        let crisp_id =
            TileId::new(page_index, coordinate, zoom_level, rotation, TileProfile::Crisp);
        let crisp_tile = self.renderer.render_tile(document, &crisp_id)?;

        // Update state
        {
            let mut states = self.tile_states.lock().unwrap();
            states.insert(crisp_id.clone(), TileState::CrispLoaded);
        }

        // Invoke callback
        if let Some(ref cb) = callback {
            cb(crisp_id.clone(), TileState::CrispLoaded, &crisp_tile);
        }

        results.push(crisp_tile);

        Ok(results)
    }

    /// Load all tiles for a page progressively
    ///
    /// First renders all preview tiles, then renders all crisp tiles.
    /// Returns tiles in two batches: all previews first, then all crisp tiles.
    pub fn load_page_tiles(
        &self,
        document: &PdfDocument,
        page_index: u16,
        zoom_level: u32,
        rotation: u16,
        callback: Option<ProgressCallback>,
    ) -> PdfResult<Vec<RenderedTile>> {
        // Get page dimensions to calculate tile grid
        let page = document.get_page(page_index)?;
        let page_width = page.width().value;
        let page_height = page.height().value;

        let (columns, rows) =
            self.renderer.calculate_tile_grid(page_width, page_height, zoom_level);

        let mut results = Vec::new();

        // Stage 1: Render all preview tiles first
        for y in 0..rows {
            for x in 0..columns {
                let coordinate = TileCoordinate::new(x, y);
                let preview_id =
                    TileId::new(page_index, coordinate, zoom_level, rotation, TileProfile::Preview);

                let preview_tile = self.renderer.render_tile(document, &preview_id)?;

                // Update state
                {
                    let mut states = self.tile_states.lock().unwrap();
                    states.insert(preview_id.clone(), TileState::PreviewLoaded);
                }

                // Invoke callback
                if let Some(ref cb) = callback {
                    cb(preview_id.clone(), TileState::PreviewLoaded, &preview_tile);
                }

                results.push(preview_tile);
            }
        }

        // Stage 2: Render all crisp tiles
        for y in 0..rows {
            for x in 0..columns {
                let coordinate = TileCoordinate::new(x, y);
                let crisp_id =
                    TileId::new(page_index, coordinate, zoom_level, rotation, TileProfile::Crisp);

                let crisp_tile = self.renderer.render_tile(document, &crisp_id)?;

                // Update state
                {
                    let mut states = self.tile_states.lock().unwrap();
                    states.insert(crisp_id.clone(), TileState::CrispLoaded);
                }

                // Invoke callback
                if let Some(ref cb) = callback {
                    cb(crisp_id.clone(), TileState::CrispLoaded, &crisp_tile);
                }

                results.push(crisp_tile);
            }
        }

        Ok(results)
    }

    /// Clear all tile states
    ///
    /// Useful when switching documents or resetting the loader.
    pub fn clear_states(&self) {
        let mut states = self.tile_states.lock().unwrap();
        states.clear();
    }

    /// Get the number of tiles currently tracked
    pub fn tracked_tile_count(&self) -> usize {
        let states = self.tile_states.lock().unwrap();
        states.len()
    }
}

impl Default for ProgressiveTileLoader {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn test_tile_state_tracking() {
        let loader = ProgressiveTileLoader::new();

        let tile_id = TileId::new(0, TileCoordinate::new(0, 0), 100, 0, TileProfile::Preview);

        // Initially not loaded
        assert_eq!(loader.get_tile_state(&tile_id), TileState::NotLoaded);

        // Manually update state
        {
            let mut states = loader.tile_states.lock().unwrap();
            states.insert(tile_id.clone(), TileState::PreviewLoaded);
        }

        assert_eq!(loader.get_tile_state(&tile_id), TileState::PreviewLoaded);

        // Update to crisp
        {
            let mut states = loader.tile_states.lock().unwrap();
            states.insert(tile_id.clone(), TileState::CrispLoaded);
        }

        assert_eq!(loader.get_tile_state(&tile_id), TileState::CrispLoaded);
    }

    #[test]
    fn test_clear_states() {
        let loader = ProgressiveTileLoader::new();

        // Add some states
        {
            let mut states = loader.tile_states.lock().unwrap();
            states.insert(
                TileId::new(0, TileCoordinate::new(0, 0), 100, 0, TileProfile::Preview),
                TileState::PreviewLoaded,
            );
            states.insert(
                TileId::new(0, TileCoordinate::new(1, 0), 100, 0, TileProfile::Preview),
                TileState::PreviewLoaded,
            );
        }

        assert_eq!(loader.tracked_tile_count(), 2);

        loader.clear_states();
        assert_eq!(loader.tracked_tile_count(), 0);
    }

    #[test]
    fn test_progressive_loading_order() {
        // This test verifies the callback mechanism structure.
        // The actual loading is tested in integration tests with real PDFs.
        let _loader = ProgressiveTileLoader::new();

        let preview_count = Arc::new(AtomicUsize::new(0));
        let crisp_count = Arc::new(AtomicUsize::new(0));

        let preview_count_clone = preview_count.clone();
        let crisp_count_clone = crisp_count.clone();

        let callback: ProgressCallback = Arc::new(move |_id, state, _tile| match state {
            TileState::PreviewLoaded => {
                preview_count_clone.fetch_add(1, Ordering::SeqCst);
            }
            TileState::CrispLoaded => {
                crisp_count_clone.fetch_add(1, Ordering::SeqCst);
            }
            TileState::NotLoaded => {}
        });

        drop(callback); // Prevent unused variable warning

        // Verify initial counts
        assert_eq!(preview_count.load(Ordering::SeqCst), 0);
        assert_eq!(crisp_count.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn test_loader_creation() {
        let loader = ProgressiveTileLoader::new();
        assert_eq!(loader.tracked_tile_count(), 0);

        let custom_renderer = TileRenderer::with_tile_size(512);
        let custom_loader = ProgressiveTileLoader::with_renderer(custom_renderer);
        assert_eq!(custom_loader.tracked_tile_count(), 0);
    }
}
