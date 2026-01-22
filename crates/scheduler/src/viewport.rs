//! Viewport-aware job priority assignment
//!
//! This module provides intelligent priority assignment for tile rendering jobs
//! based on viewport visibility. Tiles are prioritized as follows:
//! 1. Visible tiles (currently on screen) - highest priority
//! 2. Margin tiles (just outside visible area) - high priority for smooth scrolling
//! 3. Adjacent page tiles - medium priority for fast page switching
//! 4. Thumbnails - low priority
//! 5. OCR - lowest priority (runs when idle)

use crate::priority::JobPriority;

/// Viewport state for priority calculation
///
/// Represents the current viewport position and dimensions in page coordinates.
#[derive(Debug, Clone)]
pub struct Viewport {
    /// Current page index being viewed
    pub page_index: u16,

    /// Viewport X coordinate (in page coordinates)
    pub x: f32,

    /// Viewport Y coordinate (in page coordinates)
    pub y: f32,

    /// Viewport width (in page coordinates)
    pub width: f32,

    /// Viewport height (in page coordinates)
    pub height: f32,

    /// Current zoom level (percentage, e.g., 100 = 100%)
    pub zoom_level: u32,

    /// Margin size in tiles (number of tile rows/columns around visible area)
    pub margin_tiles: u32,
}

impl Viewport {
    /// Create a new viewport
    pub fn new(page_index: u16, x: f32, y: f32, width: f32, height: f32, zoom_level: u32) -> Self {
        Self {
            page_index,
            x,
            y,
            width,
            height,
            zoom_level,
            margin_tiles: 3, // Default: 3 tile margin for smooth scrolling
        }
    }

    /// Set the margin size in tiles
    pub fn with_margin_tiles(mut self, margin_tiles: u32) -> Self {
        self.margin_tiles = margin_tiles;
        self
    }
}

/// Tile position in the tile grid
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TilePosition {
    /// Page index
    pub page_index: u16,

    /// Tile X coordinate in grid
    pub tile_x: u32,

    /// Tile Y coordinate in grid
    pub tile_y: u32,

    /// Zoom level
    pub zoom_level: u32,
}

impl TilePosition {
    /// Create a new tile position
    pub fn new(page_index: u16, tile_x: u32, tile_y: u32, zoom_level: u32) -> Self {
        Self {
            page_index,
            tile_x,
            tile_y,
            zoom_level,
        }
    }
}

/// Priority calculator for tile rendering jobs
///
/// Determines the appropriate priority for tile rendering based on viewport visibility.
///
/// # Example
///
/// ```
/// use pdf_editor_scheduler::{Viewport, TilePosition, PriorityCalculator};
///
/// let viewport = Viewport::new(0, 0.0, 0.0, 800.0, 600.0, 100);
/// let calculator = PriorityCalculator::new(viewport, 256);
///
/// // Get priority for a tile
/// let tile = TilePosition::new(0, 0, 0, 100);
/// let priority = calculator.calculate_tile_priority(&tile);
/// ```
pub struct PriorityCalculator {
    viewport: Viewport,
    tile_size: u32,
}

impl PriorityCalculator {
    /// Create a new priority calculator
    ///
    /// # Arguments
    ///
    /// * `viewport` - Current viewport state
    /// * `tile_size` - Tile size in pixels (e.g., 256)
    pub fn new(viewport: Viewport, tile_size: u32) -> Self {
        Self {
            viewport,
            tile_size,
        }
    }

    /// Update the viewport
    pub fn update_viewport(&mut self, viewport: Viewport) {
        self.viewport = viewport;
    }

    /// Calculate priority for a tile rendering job
    ///
    /// Returns the appropriate priority based on viewport visibility:
    /// - Visible: Tile is currently visible in viewport
    /// - Margin: Tile is just outside viewport (for smooth scrolling)
    /// - Adjacent: Tile is on an adjacent page
    /// - Thumbnails: Job is for thumbnail generation
    /// - Ocr: Job is for OCR processing
    pub fn calculate_tile_priority(&self, tile: &TilePosition) -> JobPriority {
        // Check if tile is on current page and at current zoom level
        if tile.page_index != self.viewport.page_index
            || tile.zoom_level != self.viewport.zoom_level
        {
            // Check if tile is on adjacent page
            if tile.page_index == self.viewport.page_index.wrapping_add(1)
                || tile.page_index == self.viewport.page_index.wrapping_sub(1)
            {
                return JobPriority::Adjacent;
            }
            // Otherwise, low priority
            return JobPriority::Thumbnails;
        }

        // Calculate tile bounds in page coordinates
        let tile_size_f = self.tile_size as f32;
        let scale = self.viewport.zoom_level as f32 / 100.0;
        let scaled_tile_size = tile_size_f / scale;

        let tile_x_start = tile.tile_x as f32 * scaled_tile_size;
        let tile_y_start = tile.tile_y as f32 * scaled_tile_size;
        let tile_x_end = tile_x_start + scaled_tile_size;
        let tile_y_end = tile_y_start + scaled_tile_size;

        // Calculate viewport bounds
        let viewport_x_start = self.viewport.x;
        let viewport_y_start = self.viewport.y;
        let viewport_x_end = self.viewport.x + self.viewport.width;
        let viewport_y_end = self.viewport.y + self.viewport.height;

        // Check if tile is visible (intersects with viewport)
        if tile_x_end > viewport_x_start
            && tile_x_start < viewport_x_end
            && tile_y_end > viewport_y_start
            && tile_y_start < viewport_y_end
        {
            return JobPriority::Visible;
        }

        // Calculate margin bounds (viewport + margin_tiles)
        let margin = scaled_tile_size * self.viewport.margin_tiles as f32;
        let margin_x_start = viewport_x_start - margin;
        let margin_y_start = viewport_y_start - margin;
        let margin_x_end = viewport_x_end + margin;
        let margin_y_end = viewport_y_end + margin;

        // Check if tile is in margin (for smooth scrolling)
        if tile_x_end > margin_x_start
            && tile_x_start < margin_x_end
            && tile_y_end > margin_y_start
            && tile_y_start < margin_y_end
        {
            return JobPriority::Margin;
        }

        // Otherwise, low priority (rest of current page)
        JobPriority::Thumbnails
    }

    /// Calculate priority for a page thumbnail job
    pub fn calculate_thumbnail_priority(&self, page_index: u16) -> JobPriority {
        if page_index == self.viewport.page_index {
            // Current page thumbnail gets higher priority
            JobPriority::Margin
        } else if page_index == self.viewport.page_index.wrapping_add(1)
            || page_index == self.viewport.page_index.wrapping_sub(1)
        {
            // Adjacent page thumbnails
            JobPriority::Adjacent
        } else {
            // Other thumbnails
            JobPriority::Thumbnails
        }
    }

    /// Calculate priority for an OCR job
    ///
    /// OCR jobs always get the lowest priority (run when idle).
    pub fn calculate_ocr_priority(&self, _page_index: u16) -> JobPriority {
        JobPriority::Ocr
    }

    /// Get the current viewport
    pub fn viewport(&self) -> &Viewport {
        &self.viewport
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_viewport_creation() {
        let viewport = Viewport::new(0, 100.0, 200.0, 800.0, 600.0, 100);
        assert_eq!(viewport.page_index, 0);
        assert_eq!(viewport.x, 100.0);
        assert_eq!(viewport.y, 200.0);
        assert_eq!(viewport.width, 800.0);
        assert_eq!(viewport.height, 600.0);
        assert_eq!(viewport.zoom_level, 100);
        assert_eq!(viewport.margin_tiles, 3);
    }

    #[test]
    fn test_viewport_with_margin_tiles() {
        let viewport = Viewport::new(0, 0.0, 0.0, 800.0, 600.0, 100).with_margin_tiles(2);
        assert_eq!(viewport.margin_tiles, 2);
    }

    #[test]
    fn test_tile_position_creation() {
        let tile = TilePosition::new(0, 5, 10, 100);
        assert_eq!(tile.page_index, 0);
        assert_eq!(tile.tile_x, 5);
        assert_eq!(tile.tile_y, 10);
        assert_eq!(tile.zoom_level, 100);
    }

    #[test]
    fn test_visible_tile_priority() {
        // Viewport at origin, 800x600, tile size 256
        let viewport = Viewport::new(0, 0.0, 0.0, 800.0, 600.0, 100);
        let calculator = PriorityCalculator::new(viewport, 256);

        // Tile at origin should be visible
        let tile = TilePosition::new(0, 0, 0, 100);
        assert_eq!(
            calculator.calculate_tile_priority(&tile),
            JobPriority::Visible
        );

        // Tile at (1, 0) should be visible (within 800px width)
        let tile = TilePosition::new(0, 1, 0, 100);
        assert_eq!(
            calculator.calculate_tile_priority(&tile),
            JobPriority::Visible
        );

        // Tile at (0, 1) should be visible (within 600px height)
        let tile = TilePosition::new(0, 0, 1, 100);
        assert_eq!(
            calculator.calculate_tile_priority(&tile),
            JobPriority::Visible
        );
    }

    #[test]
    fn test_margin_tile_priority() {
        // Tile at (-1, 0) should be in margin (1 tile to the left)
        // Since we can't have negative tile coords, test with viewport offset
        // Viewport offset by 1 tile in both directions
        let viewport = Viewport::new(0, 256.0, 256.0, 800.0, 600.0, 100);
        let calculator = PriorityCalculator::new(viewport, 256);

        // Tile at (0, 0) should be in margin (top-left of viewport)
        let tile = TilePosition::new(0, 0, 0, 100);
        assert_eq!(
            calculator.calculate_tile_priority(&tile),
            JobPriority::Margin
        );

        // Tile at (5, 0) should be in margin (to the right of viewport)
        let tile = TilePosition::new(0, 5, 0, 100);
        assert_eq!(
            calculator.calculate_tile_priority(&tile),
            JobPriority::Margin
        );
    }

    #[test]
    fn test_adjacent_page_priority() {
        let viewport = Viewport::new(5, 0.0, 0.0, 800.0, 600.0, 100);
        let calculator = PriorityCalculator::new(viewport, 256);

        // Tile on page 4 (previous page) should be adjacent priority
        let tile = TilePosition::new(4, 0, 0, 100);
        assert_eq!(
            calculator.calculate_tile_priority(&tile),
            JobPriority::Adjacent
        );

        // Tile on page 6 (next page) should be adjacent priority
        let tile = TilePosition::new(6, 0, 0, 100);
        assert_eq!(
            calculator.calculate_tile_priority(&tile),
            JobPriority::Adjacent
        );

        // Tile on page 10 (distant page) should be thumbnails priority
        let tile = TilePosition::new(10, 0, 0, 100);
        assert_eq!(
            calculator.calculate_tile_priority(&tile),
            JobPriority::Thumbnails
        );
    }

    #[test]
    fn test_different_zoom_level() {
        let viewport = Viewport::new(0, 0.0, 0.0, 800.0, 600.0, 100);
        let calculator = PriorityCalculator::new(viewport, 256);

        // Tile at same position but different zoom should be thumbnails priority
        let tile = TilePosition::new(0, 0, 0, 200);
        assert_eq!(
            calculator.calculate_tile_priority(&tile),
            JobPriority::Thumbnails
        );
    }

    #[test]
    fn test_zoomed_viewport() {
        // Viewport at 200% zoom
        let viewport = Viewport::new(0, 0.0, 0.0, 800.0, 600.0, 200);
        let calculator = PriorityCalculator::new(viewport, 256);

        // At 200% zoom, tile size in page coords is 128px
        // Tile at (0, 0) should be visible
        let tile = TilePosition::new(0, 0, 0, 200);
        assert_eq!(
            calculator.calculate_tile_priority(&tile),
            JobPriority::Visible
        );

        // Tile at (6, 0) should be visible (6 * 128 = 768 < 800)
        let tile = TilePosition::new(0, 6, 0, 200);
        assert_eq!(
            calculator.calculate_tile_priority(&tile),
            JobPriority::Visible
        );

        // Tile at (7, 0) should be in margin (7 * 128 = 896 > 800)
        let tile = TilePosition::new(0, 7, 0, 200);
        assert_eq!(
            calculator.calculate_tile_priority(&tile),
            JobPriority::Margin
        );
    }

    #[test]
    fn test_thumbnail_priority() {
        let viewport = Viewport::new(5, 0.0, 0.0, 800.0, 600.0, 100);
        let calculator = PriorityCalculator::new(viewport, 256);

        // Current page thumbnail
        assert_eq!(
            calculator.calculate_thumbnail_priority(5),
            JobPriority::Margin
        );

        // Adjacent page thumbnails
        assert_eq!(
            calculator.calculate_thumbnail_priority(4),
            JobPriority::Adjacent
        );
        assert_eq!(
            calculator.calculate_thumbnail_priority(6),
            JobPriority::Adjacent
        );

        // Distant page thumbnail
        assert_eq!(
            calculator.calculate_thumbnail_priority(10),
            JobPriority::Thumbnails
        );
    }

    #[test]
    fn test_ocr_priority() {
        let viewport = Viewport::new(0, 0.0, 0.0, 800.0, 600.0, 100);
        let calculator = PriorityCalculator::new(viewport, 256);

        // OCR jobs always get lowest priority
        assert_eq!(calculator.calculate_ocr_priority(0), JobPriority::Ocr);
        assert_eq!(calculator.calculate_ocr_priority(10), JobPriority::Ocr);
    }

    #[test]
    fn test_update_viewport() {
        let viewport = Viewport::new(0, 0.0, 0.0, 800.0, 600.0, 100);
        let mut calculator = PriorityCalculator::new(viewport, 256);

        let tile = TilePosition::new(0, 0, 0, 100);
        assert_eq!(
            calculator.calculate_tile_priority(&tile),
            JobPriority::Visible
        );

        // Update viewport to different page
        let new_viewport = Viewport::new(5, 0.0, 0.0, 800.0, 600.0, 100);
        calculator.update_viewport(new_viewport);

        // Same tile should now be thumbnails priority (different page)
        assert_eq!(
            calculator.calculate_tile_priority(&tile),
            JobPriority::Thumbnails
        );
    }

    #[test]
    fn test_viewport_offset() {
        // Viewport offset from origin
        let viewport = Viewport::new(0, 500.0, 300.0, 800.0, 600.0, 100);
        let calculator = PriorityCalculator::new(viewport, 256);

        // Tile at (0, 0) is at position 0-256, viewport starts at 500
        // Margin extends back to 500-256=244, so tile at 0-256 overlaps with margin
        // Should be in margin priority
        let tile = TilePosition::new(0, 0, 0, 100);
        assert_eq!(
            calculator.calculate_tile_priority(&tile),
            JobPriority::Margin
        );

        // Tile at (2, 1) is at position 512-768 x 256-512
        // Viewport is 500-1300 x 300-900
        // Should be visible (overlaps)
        let tile = TilePosition::new(0, 2, 1, 100);
        assert_eq!(
            calculator.calculate_tile_priority(&tile),
            JobPriority::Visible
        );
    }

    #[test]
    fn test_viewport_accessor() {
        let viewport = Viewport::new(0, 0.0, 0.0, 800.0, 600.0, 100);
        let calculator = PriorityCalculator::new(viewport.clone(), 256);

        let retrieved = calculator.viewport();
        assert_eq!(retrieved.page_index, viewport.page_index);
        assert_eq!(retrieved.x, viewport.x);
        assert_eq!(retrieved.y, viewport.y);
    }

    #[test]
    fn test_margin_tiles_configuration() {
        // Test with 2-tile margin
        let viewport = Viewport::new(0, 256.0, 256.0, 800.0, 600.0, 100).with_margin_tiles(2);
        let calculator = PriorityCalculator::new(viewport, 256);

        // Tile at (0, 0) is 2 tiles away from visible area
        // Should be in margin with 2-tile margin setting
        let tile = TilePosition::new(0, 0, 0, 100);
        assert_eq!(
            calculator.calculate_tile_priority(&tile),
            JobPriority::Margin
        );
    }
}
