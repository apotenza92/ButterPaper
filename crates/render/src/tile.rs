//! Tile-based page rendering system
//!
//! Divides PDF pages into fixed-size tiles for efficient rendering and caching.

use crate::pdf::{PdfDocument, PdfError, PdfResult};
use pdfium_render::prelude::*;
use std::hash::{Hash, Hasher};

/// Fixed tile size in pixels (256x256)
pub const TILE_SIZE: u32 = 256;

/// Tile coordinates within a page
///
/// Represents the position of a tile in the page's tile grid.
/// (0, 0) is the top-left tile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TileCoordinate {
    pub x: u32,
    pub y: u32,
}

impl TileCoordinate {
    /// Create a new tile coordinate
    pub fn new(x: u32, y: u32) -> Self {
        Self { x, y }
    }

    /// Convert tile coordinate to pixel offset (top-left corner)
    pub fn to_pixel_offset(&self, tile_size: u32) -> (u32, u32) {
        (self.x * tile_size, self.y * tile_size)
    }
}

/// Tile identity and metadata
///
/// Uniquely identifies a tile within a document for caching purposes.
/// The content hash will be added in Phase 3 for cache invalidation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TileId {
    /// Page index (0-based)
    pub page_index: u16,

    /// Tile coordinate within the page
    pub coordinate: TileCoordinate,

    /// Zoom level (represented as percentage, e.g., 100 = 100%)
    pub zoom_level: u32,

    /// Page rotation in degrees (0, 90, 180, 270)
    pub rotation: u16,

    /// Render profile ("preview" or "crisp")
    pub profile: TileProfile,
}

impl TileId {
    /// Create a new tile ID
    pub fn new(
        page_index: u16,
        coordinate: TileCoordinate,
        zoom_level: u32,
        rotation: u16,
        profile: TileProfile,
    ) -> Self {
        Self {
            page_index,
            coordinate,
            zoom_level,
            rotation,
            profile,
        }
    }

    /// Compute a simple hash for this tile ID (for cache keys)
    pub fn cache_key(&self) -> u64 {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.hash(&mut hasher);
        hasher.finish()
    }
}

impl Hash for TileId {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.page_index.hash(state);
        self.coordinate.hash(state);
        self.zoom_level.hash(state);
        self.rotation.hash(state);
        self.profile.hash(state);
    }
}

/// Render profile for tiles
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TileProfile {
    /// Fast, lower fidelity rendering for quick previews
    Preview,

    /// High fidelity rendering for final display
    Crisp,
}

/// Rendered tile data
///
/// Contains the raw RGBA pixel data for a rendered tile.
#[derive(Debug, Clone)]
pub struct RenderedTile {
    /// Tile identity
    pub id: TileId,

    /// Pixel data in RGBA format (4 bytes per pixel)
    pub pixels: Vec<u8>,

    /// Actual width of the tile in pixels (may be smaller than TILE_SIZE at edges)
    pub width: u32,

    /// Actual height of the tile in pixels (may be smaller than TILE_SIZE at edges)
    pub height: u32,
}

impl RenderedTile {
    /// Get the size of the pixel data in bytes
    pub fn byte_size(&self) -> usize {
        self.pixels.len()
    }

    /// Check if the tile is fully opaque
    pub fn is_opaque(&self) -> bool {
        // Check if all alpha values are 255
        self.pixels.chunks_exact(4).all(|rgba| rgba[3] == 255)
    }
}

/// Tile renderer
///
/// Renders PDF pages into fixed-size tiles using PDFium.
pub struct TileRenderer {
    /// Tile size in pixels
    tile_size: u32,
}

impl TileRenderer {
    /// Create a new tile renderer with the default tile size
    pub fn new() -> Self {
        Self {
            tile_size: TILE_SIZE,
        }
    }

    /// Create a new tile renderer with a custom tile size
    pub fn with_tile_size(tile_size: u32) -> Self {
        Self { tile_size }
    }

    /// Get the tile size
    pub fn tile_size(&self) -> u32 {
        self.tile_size
    }

    /// Calculate the tile grid dimensions for a page
    ///
    /// Returns (columns, rows) - the number of tiles in each dimension.
    pub fn calculate_tile_grid(
        &self,
        page_width: f32,
        page_height: f32,
        zoom_level: u32,
    ) -> (u32, u32) {
        // Apply zoom to page dimensions
        let zoomed_width = (page_width * (zoom_level as f32 / 100.0)) as u32;
        let zoomed_height = (page_height * (zoom_level as f32 / 100.0)) as u32;

        // Calculate number of tiles needed
        let columns = zoomed_width.div_ceil(self.tile_size);
        let rows = zoomed_height.div_ceil(self.tile_size);

        (columns, rows)
    }

    /// Render a single tile from a PDF page
    ///
    /// # Arguments
    /// * `document` - The PDF document
    /// * `tile_id` - Identity of the tile to render
    ///
    /// # Returns
    /// A `RenderedTile` with the pixel data or an error
    pub fn render_tile(&self, document: &PdfDocument, tile_id: &TileId) -> PdfResult<RenderedTile> {
        // Get the page
        let page = document.get_page(tile_id.page_index)?;

        // Get page dimensions
        let page_width = page.width().value;
        let page_height = page.height().value;

        // Apply zoom to get actual render dimensions
        let zoom_factor = tile_id.zoom_level as f32 / 100.0;
        let render_width = (page_width * zoom_factor) as u32;
        let render_height = (page_height * zoom_factor) as u32;

        // Calculate tile position and size
        let (tile_x, tile_y) = tile_id.coordinate.to_pixel_offset(self.tile_size);
        let tile_width = self.tile_size.min(render_width.saturating_sub(tile_x));
        let tile_height = self.tile_size.min(render_height.saturating_sub(tile_y));

        // Ensure tile is within bounds
        if tile_width == 0 || tile_height == 0 {
            return Err(PdfError::RenderError(format!(
                "Tile coordinate ({}, {}) is out of bounds for page {} at zoom {}%",
                tile_id.coordinate.x, tile_id.coordinate.y, tile_id.page_index, tile_id.zoom_level
            )));
        }

        // Configure render settings based on profile
        let render_config = match tile_id.profile {
            TileProfile::Preview => {
                // Preview: faster rendering with lower quality
                PdfRenderConfig::new()
                    .set_target_width(render_width as i32)
                    .set_target_height(render_height as i32)
                    .render_form_data(false)
            }
            TileProfile::Crisp => {
                // Crisp: high quality rendering with all features
                PdfRenderConfig::new()
                    .set_target_width(render_width as i32)
                    .set_target_height(render_height as i32)
                    .render_form_data(true)
                    .use_print_quality(true)
            }
        };

        // Render the entire page at the target zoom level
        let bitmap = page
            .render_with_config(&render_config)
            .map_err(|e| PdfError::RenderError(e.to_string()))?;

        // Convert bitmap to RGBA
        let full_page_rgba = bitmap.as_rgba_bytes();

        // Extract the tile region from the full page render
        let mut tile_pixels = Vec::with_capacity((tile_width * tile_height * 4) as usize);

        for y in 0..tile_height {
            let src_y = tile_y + y;
            let src_offset = (src_y * render_width + tile_x) as usize * 4;
            let src_end = src_offset + (tile_width as usize * 4);

            if src_end <= full_page_rgba.len() {
                tile_pixels.extend_from_slice(&full_page_rgba[src_offset..src_end]);
            } else {
                // Fill with white if we're at the edge
                tile_pixels.extend(vec![255u8; (tile_width * 4) as usize]);
            }
        }

        Ok(RenderedTile {
            id: tile_id.clone(),
            pixels: tile_pixels,
            width: tile_width,
            height: tile_height,
        })
    }

    /// Render all tiles for a page at a given zoom level
    ///
    /// Returns a vector of all rendered tiles for the page.
    pub fn render_page_tiles(
        &self,
        document: &PdfDocument,
        page_index: u16,
        zoom_level: u32,
        profile: TileProfile,
    ) -> PdfResult<Vec<RenderedTile>> {
        // Get page dimensions
        let page = document.get_page(page_index)?;
        let page_width = page.width().value;
        let page_height = page.height().value;

        // Calculate tile grid
        let (columns, rows) = self.calculate_tile_grid(page_width, page_height, zoom_level);

        // Render all tiles
        let mut tiles = Vec::with_capacity((columns * rows) as usize);

        for y in 0..rows {
            for x in 0..columns {
                let tile_id = TileId::new(
                    page_index,
                    TileCoordinate::new(x, y),
                    zoom_level,
                    0, // No rotation for now
                    profile,
                );

                let tile = self.render_tile(document, &tile_id)?;
                tiles.push(tile);
            }
        }

        Ok(tiles)
    }
}

impl Default for TileRenderer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tile_coordinate() {
        let coord = TileCoordinate::new(2, 3);
        assert_eq!(coord.x, 2);
        assert_eq!(coord.y, 3);

        let (px, py) = coord.to_pixel_offset(256);
        assert_eq!(px, 512);
        assert_eq!(py, 768);
    }

    #[test]
    fn test_tile_id_cache_key() {
        let id1 = TileId::new(
            0,
            TileCoordinate::new(1, 2),
            100,
            0,
            TileProfile::Preview,
        );
        let id2 = TileId::new(
            0,
            TileCoordinate::new(1, 2),
            100,
            0,
            TileProfile::Preview,
        );
        let id3 = TileId::new(
            0,
            TileCoordinate::new(1, 2),
            100,
            0,
            TileProfile::Crisp,
        );

        // Same IDs should produce same cache key
        assert_eq!(id1.cache_key(), id2.cache_key());

        // Different IDs should (very likely) produce different cache keys
        assert_ne!(id1.cache_key(), id3.cache_key());
    }

    #[test]
    fn test_tile_renderer_creation() {
        let renderer = TileRenderer::new();
        assert_eq!(renderer.tile_size(), TILE_SIZE);

        let renderer = TileRenderer::with_tile_size(512);
        assert_eq!(renderer.tile_size(), 512);
    }

    #[test]
    fn test_calculate_tile_grid() {
        let renderer = TileRenderer::new();

        // 612x792 points is US Letter size
        // At 100% zoom, this is 612x792 pixels
        // Tiles are 256x256
        // Should need 3 columns (256*3 = 768 > 612) and 4 rows (256*4 = 1024 > 792)
        let (cols, rows) = renderer.calculate_tile_grid(612.0, 792.0, 100);
        assert_eq!(cols, 3);
        assert_eq!(rows, 4);

        // At 200% zoom, page is 1224x1584
        // Should need 5 columns and 7 rows
        let (cols, rows) = renderer.calculate_tile_grid(612.0, 792.0, 200);
        assert_eq!(cols, 5);
        assert_eq!(rows, 7);
    }

    #[test]
    fn test_tile_profile_equality() {
        assert_eq!(TileProfile::Preview, TileProfile::Preview);
        assert_eq!(TileProfile::Crisp, TileProfile::Crisp);
        assert_ne!(TileProfile::Preview, TileProfile::Crisp);
    }

    #[test]
    fn test_rendered_tile_byte_size() {
        let tile = RenderedTile {
            id: TileId::new(
                0,
                TileCoordinate::new(0, 0),
                100,
                0,
                TileProfile::Preview,
            ),
            pixels: vec![255u8; 256 * 256 * 4],
            width: 256,
            height: 256,
        };

        assert_eq!(tile.byte_size(), 256 * 256 * 4);
    }

    #[test]
    fn test_rendered_tile_is_opaque() {
        // Fully opaque tile
        let opaque_tile = RenderedTile {
            id: TileId::new(
                0,
                TileCoordinate::new(0, 0),
                100,
                0,
                TileProfile::Preview,
            ),
            pixels: vec![255u8; 256 * 256 * 4],
            width: 256,
            height: 256,
        };
        assert!(opaque_tile.is_opaque());

        // Tile with some transparency
        let mut transparent_pixels = vec![255u8; 256 * 256 * 4];
        transparent_pixels[3] = 128; // Set first pixel's alpha to 128
        let transparent_tile = RenderedTile {
            id: TileId::new(
                0,
                TileCoordinate::new(0, 0),
                100,
                0,
                TileProfile::Preview,
            ),
            pixels: transparent_pixels,
            width: 256,
            height: 256,
        };
        assert!(!transparent_tile.is_opaque());
    }
}
